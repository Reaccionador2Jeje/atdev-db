use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use tiny_http::{Response, Server, StatusCode};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Table {
    name: String,
    columns: Vec<String>,
    types: Vec<String>,
    rows: Vec<Vec<Value>>,
}

struct Database {
    directory: PathBuf,
}

impl Database {
    fn new() -> Result<Self> {
        let directory = env::var("ATDEV_DB_PATH").unwrap_or_else(|_| "./atdev-data".into());
        Self::at(PathBuf::from(directory))
    }

    fn at(directory: PathBuf) -> Result<Self> {
        fs::create_dir_all(&directory).context("could not create database directory")?;
        Ok(Self { directory })
    }

    fn table_path(&self, name: &str) -> Result<PathBuf> {
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            bail!("invalid table name");
        }
        Ok(self.directory.join(format!("{name}.json")))
    }

    fn load(&self, name: &str) -> Result<Table> {
        let path = self.table_path(name)?;
        let bytes = fs::read(&path).with_context(|| format!("table '{name}' does not exist"))?;
        serde_json::from_slice(&bytes).context("table file is corrupted or has an unsupported format")
    }

    fn save(&self, table: &Table) -> Result<()> {
        let path = self.table_path(&table.name)?;
        let temp = path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(table)?;
        fs::write(&temp, bytes)?;
        fs::rename(temp, path)?;
        Ok(())
    }

    fn execute(&self, sql: &str) -> Result<String> {
        let mut output = Vec::new();
        for statement in split_statements(sql) {
            let tokens = tokenize(statement)?;
            if tokens.is_empty() { continue; }
            let result = match tokens[0].to_ascii_uppercase().as_str() {
                "CREATE" => self.create_table(&tokens),
                "INSERT" => self.insert(&tokens),
                "SELECT" => self.select(&tokens),
                "UPDATE" => self.update(&tokens),
                "DELETE" => self.delete(&tokens),
                other => bail!("unsupported statement '{other}' (supported: CREATE, INSERT, SELECT, UPDATE, DELETE)"),
            }?;
            if !result.is_empty() { output.push(result); }
        }
        Ok(output.join("\n\n"))
    }

    fn create_table(&self, t: &[String]) -> Result<String> {
        if t.len() < 6 || !eq(&t[1], "TABLE") { bail!("syntax: CREATE TABLE name (column TYPE, ...)"); }
        let name = t[2].clone();
        let path = self.table_path(&name)?;
        if path.exists() { bail!("table '{name}' already exists"); }
        if t[3] != "(" || *t.last().unwrap() != ")" { bail!("syntax: CREATE TABLE name (column TYPE, ...)"); }
        let mut columns = Vec::new();
        let mut types = Vec::new();
        let mut i = 4;
        while i < t.len() - 1 {
            let col = t[i].clone();
            let kind = t.get(i + 1).ok_or_else(|| anyhow!("missing type for column '{col}'"))?.to_ascii_uppercase();
            if !["TEXT", "INTEGER", "REAL", "BOOLEAN"].contains(&kind.as_str()) {
                bail!("unsupported type '{kind}' for column '{col}'");
            }
            if columns.iter().any(|c| c == &col) { bail!("duplicate column '{col}'"); }
            columns.push(col);
            types.push(kind);
            i += 2;
            if i < t.len() - 1 {
                if t[i] != "," { bail!("expected ',' between columns"); }
                i += 1;
            }
        }
        if columns.is_empty() { bail!("a table must have at least one column"); }
        self.save(&Table { name, columns, types, rows: Vec::new() })?;
        Ok("CREATE TABLE: table created".into())
    }

    fn insert(&self, t: &[String]) -> Result<String> {
        if t.len() < 7 || !eq(&t[1], "INTO") || !eq(&t[2], "VALUES") && !eq(&t[3], "VALUES") {
            bail!("syntax: INSERT INTO table VALUES (value, ...)");
        }
        let (name, values_at) = if eq(&t[2], "VALUES") { (&t[1], 3) } else { (&t[2], 4) };
        if t.get(values_at).map(String::as_str) != Some("(") || t.last().map(String::as_str) != Some(")") {
            bail!("syntax: INSERT INTO table VALUES (value, ...)");
        }
        let values = parse_value_list(&t[values_at + 1..t.len() - 1])?;
        let mut table = self.load(name)?;
        if values.len() != table.columns.len() {
            bail!("expected {} values, received {}", table.columns.len(), values.len());
        }
        for (index, value) in values.iter().enumerate() {
            validate_type(value, &table.types[index]).with_context(|| format!("invalid value for column '{}'", table.columns[index]))?;
        }
        table.rows.push(values);
        self.save(&table)?;
        Ok(format!("INSERT: 1 row added to '{}'", name))
    }

    fn select(&self, t: &[String]) -> Result<String> {
        let from = find_keyword(t, "FROM").ok_or_else(|| anyhow!("syntax: SELECT columns FROM table [WHERE column = value]"))?;
        if from < 2 || from + 1 >= t.len() { bail!("syntax: SELECT columns FROM table [WHERE column = value]"); }
        let mut table = self.load(&t[from + 1])?;
        let selected: Vec<usize> = if t[1] == "*" {
            (0..table.columns.len()).collect()
        } else {
            t[1..from].iter().filter(|x| *x != ",").map(|col| {
                table.columns.iter().position(|c| c == col).ok_or_else(|| anyhow!("unknown column '{col}'"))
            }).collect::<Result<Vec<_>>>()?
        };
        if let Some(where_at) = find_keyword(t, "WHERE") {
            let (column, op, value) = parse_condition(t, where_at + 1)?;
            let index = table.columns.iter().position(|c| c == column).ok_or_else(|| anyhow!("unknown column '{column}'"))?;
            table.rows.retain(|row| compare(&row[index], op, &value));
        }
        let headers: Vec<String> = selected.iter().map(|i| table.columns[*i].clone()).collect();
        let mut output = vec![headers.join(" | "), "-".repeat(headers.join(" | ").len())];
        for row in &table.rows {
            output.push(selected.iter().map(|i| display(&row[*i])).collect::<Vec<_>>().join(" | "));
        }
        output.push(format!("({} rows)", table.rows.len()));
        Ok(output.join("\n"))
    }

    fn update(&self, t: &[String]) -> Result<String> {
        if t.len() < 6 || !eq(&t[2], "SET") { bail!("syntax: UPDATE table SET column = value [WHERE column = value]"); }
        let where_at = find_keyword(t, "WHERE").unwrap_or(t.len());
        let assignments = parse_assignments(&t[3..where_at])?;
        let mut table = self.load(&t[1])?;
        let mut parsed = Vec::new();
        for (column, value) in assignments {
            let index = table.columns.iter().position(|c| c == &column).ok_or_else(|| anyhow!("unknown column '{column}'"))?;
            validate_type(&value, &table.types[index])?;
            parsed.push((index, value));
        }
        let condition = if where_at < t.len() {
            let (column, op, value) = parse_condition(t, where_at + 1)?;
            let index = table.columns.iter().position(|c| c == column).ok_or_else(|| anyhow!("unknown column '{column}'"))?;
            Some((index, op.to_string(), value))
        } else { None };
        let mut changed = 0;
        for row in &mut table.rows {
            if condition.as_ref().map(|(i, op, v)| compare(&row[*i], op, v)).unwrap_or(true) {
                for (i, value) in &parsed { row[*i] = value.clone(); }
                changed += 1;
            }
        }
        self.save(&table)?;
        Ok(format!("UPDATE: {changed} row(s) changed"))
    }

    fn delete(&self, t: &[String]) -> Result<String> {
        if t.len() < 3 || !eq(&t[1], "FROM") { bail!("syntax: DELETE FROM table [WHERE column = value]"); }
        let mut table = self.load(&t[2])?;
        let before = table.rows.len();
        if let Some(where_at) = find_keyword(t, "WHERE") {
            let (column, op, value) = parse_condition(t, where_at + 1)?;
            let index = table.columns.iter().position(|c| c == column).ok_or_else(|| anyhow!("unknown column '{column}'"))?;
            table.rows.retain(|row| !compare(&row[index], op, &value));
        } else {
            table.rows.clear();
        }
        let removed = before - table.rows.len();
        self.save(&table)?;
        Ok(format!("DELETE: {removed} row(s) removed"))
    }
}


fn split_statements(sql: &str) -> Vec<&str> {
    let mut statements = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    let mut chars = sql.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        if ch == '\'' {
            if quoted && chars.peek().map(|(_, next)| *next) == Some('\'') {
                chars.next();
            } else {
                quoted = !quoted;
            }
        } else if ch == ';' && !quoted {
            let part = sql[start..index].trim();
            if !part.is_empty() { statements.push(part); }
            start = index + 1;
        }
    }
    let tail = sql[start..].trim();
    if !tail.is_empty() { statements.push(tail); }
    statements
}

fn eq(a: &str, b: &str) -> bool { a.eq_ignore_ascii_case(b) }

fn tokenize(sql: &str) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut chars = sql.chars().peekable();
    let mut quote = false;
    while let Some(ch) = chars.next() {
        if ch == '\'' {
            current.push(ch);
            if quote && chars.peek() == Some(&'\'') {
                current.push(chars.next().unwrap());
            } else {
                quote = !quote;
            }
        } else if !quote && (ch.is_whitespace() || ch == ',' || ch == '(' || ch == ')' || ch == ';' || ch == '=' || ch == '>' || ch == '<') {
            if !current.is_empty() { out.push(std::mem::take(&mut current)); }
            if ch == '>' || ch == '<' {
                let mut operator = ch.to_string();
                if chars.peek() == Some(&'=') { operator.push(chars.next().unwrap()); }
                out.push(operator);
            } else if ch == ',' || ch == '(' || ch == ')' || ch == '=' {
                out.push(ch.to_string());
            }
        } else {
            current.push(ch);
        }
    }
    if quote { bail!("unterminated string literal"); }
    if !current.is_empty() { out.push(current); }
    Ok(out)
}

fn parse_value(token: &str) -> Result<Value> {
    if token.starts_with('\'') && token.ends_with('\'') && token.len() >= 2 {
        return Ok(Value::String(token[1..token.len()-1].replace("''", "'")));
    }
    if token.eq_ignore_ascii_case("true") { return Ok(Value::Bool(true)); }
    if token.eq_ignore_ascii_case("false") { return Ok(Value::Bool(false)); }
    if let Ok(n) = token.parse::<i64>() { return Ok(Value::from(n)); }
    if let Ok(n) = token.parse::<f64>() { return serde_json::Number::from_f64(n).map(Value::Number).ok_or_else(|| anyhow!("invalid number")); }
    bail!("invalid SQL value '{token}'")
}

fn parse_value_list(tokens: &[String]) -> Result<Vec<Value>> {
    let mut values = Vec::new();
    let mut expect_value = true;
    for token in tokens {
        if token == "," {
            if expect_value { bail!("unexpected comma in values"); }
            expect_value = true;
        } else {
            if !expect_value { bail!("expected comma between values"); }
            values.push(parse_value(token)?);
            expect_value = false;
        }
    }
    if expect_value { bail!("missing value"); }
    Ok(values)
}

fn validate_type(value: &Value, kind: &str) -> Result<()> {
    let valid = match kind {
        "TEXT" => value.is_string(),
        "INTEGER" => value.as_i64().is_some(),
        "REAL" => value.is_number(),
        "BOOLEAN" => value.is_boolean(),
        _ => false,
    };
    if !valid { bail!("expected {kind}"); }
    Ok(())
}

fn find_keyword(tokens: &[String], keyword: &str) -> Option<usize> {
    tokens.iter().position(|t| eq(t, keyword))
}

fn parse_condition<'a>(t: &'a [String], start: usize) -> Result<(&'a str, &'a str, Value)> {
    if t.len() < start + 3 { bail!("WHERE expects: column = value"); }
    let column = t[start].as_str();
    let op = t[start + 1].as_str();
    if !["=", ">", "<", ">=", "<="].contains(&op) { bail!("supported WHERE operators: =, >, <, >=, <="); }
    Ok((column, op, parse_value(&t[start + 2])?))
}

fn compare(left: &Value, op: &str, right: &Value) -> bool {
    if op == "=" { return left == right; }
    let ordering = if let (Some(a), Some(b)) = (left.as_f64(), right.as_f64()) {
        a.partial_cmp(&b)
    } else if let (Some(a), Some(b)) = (left.as_str(), right.as_str()) {
        Some(a.cmp(b))
    } else { None };
    match (op, ordering) {
        (">", Some(std::cmp::Ordering::Greater)) | ("<", Some(std::cmp::Ordering::Less)) |
        (">=", Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)) |
        ("<=", Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)) => true,
        _ => false,
    }
}

fn parse_assignments(tokens: &[String]) -> Result<Vec<(String, Value)>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if i + 2 >= tokens.len() || tokens[i + 1] != "=" { bail!("SET expects column = value"); }
        out.push((tokens[i].clone(), parse_value(&tokens[i + 2])?));
        i += 3;
        if i < tokens.len() {
            if tokens[i] != "," { bail!("expected comma between assignments"); }
            i += 1;
        }
    }
    if out.is_empty() { bail!("at least one assignment is required"); }
    Ok(out)
}

fn display(value: &Value) -> String {
    match value { Value::String(s) => s.clone(), _ => value.to_string() }
}

#[derive(Deserialize)]
struct QueryRequest {
    database: String,
    sql: String,
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() { return false; }
    let mut difference = 0u8;
    for (a, b) in left.iter().zip(right.iter()) { difference |= a ^ b; }
    difference == 0
}

fn run_server() -> Result<()> {
    let address = env::var("ATDEV_DB_BIND").unwrap_or_else(|_| "127.0.0.1:8788".into());
    let root = PathBuf::from(env::var("ATDEV_DB_PATH").unwrap_or_else(|_| "./atdev-data".into()));
    fs::create_dir_all(&root)?;
    let token = env::var("ATDEV_DB_TOKEN").ok().filter(|value| !value.trim().is_empty());
    let loopback = address.starts_with("127.0.0.1:")
        || address.starts_with("localhost:")
        || address.starts_with("[::1]:")
        || address.starts_with("::1:");
    if !loopback && token.is_none() {
        bail!("ATDEV_DB_TOKEN is required when binding outside loopback interfaces");
    }
    let server = Server::http(&address).map_err(|e| anyhow!("could not bind AtDev DB server: {e}"))?;
    println!("AtDev DB API listening on http://{address}");
    for mut request in server.incoming_requests() {
        let url = request.url().to_string();
        if request.method().as_str() == "GET" && url == "/health" {
            let response = Response::from_string(r#"{"status":"ok","service":"atdev-db"}"#)
                .with_header(tiny_http::Header::from_bytes("Content-Type", "application/json").map_err(|_| anyhow!("invalid response header"))?);
            let _ = request.respond(response);
            continue;
        }
        if request.method().as_str() != "POST" || url != "/v1/query" {
            let response = Response::from_string(r#"{"error":"not found"}"#)
                .with_status_code(StatusCode(404))
                .with_header(tiny_http::Header::from_bytes("Content-Type", "application/json").map_err(|_| anyhow!("invalid response header"))?);
            let _ = request.respond(response);
            continue;
        }
        if let Some(expected_token) = token.as_deref() {
            let expected = format!("Bearer {expected_token}");
            let supplied = request.headers().iter()
                .find(|header| header.field.equiv("Authorization"))
                .map(|header| header.value.as_str())
                .unwrap_or("");
            if !constant_time_equal(supplied.as_bytes(), expected.as_bytes()) {
                let response = Response::from_string(r#"{"ok":false,"error":"unauthorized"}"#)
                    .with_status_code(StatusCode(401))
                    .with_header(tiny_http::Header::from_bytes("Content-Type", "application/json").map_err(|_| anyhow!("invalid response header"))?);
                let _ = request.respond(response);
                continue;
            }
        }
        let mut body = String::new();
        let result = request.as_reader().take(1_048_577).read_to_string(&mut body)
            .and_then(|_| if body.len() > 1_048_576 { Err(io::Error::new(io::ErrorKind::InvalidData, "request body exceeds 1 MiB")) } else { Ok(body.len()) })
            .context("could not read request body")
            .and_then(|_| serde_json::from_str::<QueryRequest>(&body).context("invalid JSON request"));
        let response_body = match result {
            Ok(input) if !input.sql.trim().is_empty() && input.sql.len() <= 10_000
                && !input.database.is_empty() && input.database.len() <= 80
                && input.database.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') =>
            {
                match Database::at(root.join(&input.database)).and_then(|db| db.execute(&input.sql)) {
                    Ok(output) => serde_json::json!({"ok": true, "result": output}).to_string(),
                    Err(error) => serde_json::json!({"ok": false, "error": format!("{error:#}")}).to_string(),
                }
            }
            Ok(_) => serde_json::json!({"ok": false, "error": "invalid query or database name"}).to_string(),
            Err(error) => serde_json::json!({"ok": false, "error": format!("{error:#}")}).to_string(),
        };
        let response = Response::from_string(response_body)
            .with_header(tiny_http::Header::from_bytes("Content-Type", "application/json").map_err(|_| anyhow!("invalid response header"))?);
        let _ = request.respond(response);
    }
    Ok(())
}

fn main() -> Result<()> {
    if env::args().any(|arg| arg == "--server") { return run_server(); }
    let db = Database::new()?;
    println!("AtDev DB 0.1.0-alpha — experimental SQL database");
    println!("Data directory: {} | type .exit to quit", db.directory.display());
    let stdin = io::stdin();
    loop {
        print!("atdev-db> ");
        io::stdout().flush()?;
        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 { break; }
        let sql = line.trim();
        if sql.is_empty() { continue; }
        if sql.eq_ignore_ascii_case(".exit") || sql.eq_ignore_ascii_case(".quit") { break; }
        match db.execute(sql) {
            Ok(output) if !output.is_empty() => println!("{output}"),
            Ok(_) => {},
            Err(error) => eprintln!("ERROR: {error:#}"),
        }
    }
    println!("Goodbye.");
    Ok(())
}
