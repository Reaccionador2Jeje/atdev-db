# AtDev DB

An independent, open-source relational SQL database engine developed by AtDev Dev.

AtDev DB starts with a deliberately small SQL dialect and a durable JSON-backed table store. It is an experimental alpha, not yet a production database or a PostgreSQL replacement.

## Features in 0.1.0-alpha
- Interactive SQL command-line shell.
- `CREATE TABLE`, `INSERT INTO ... VALUES`, `SELECT`, `UPDATE` and `DELETE`.
- Basic column types: TEXT, INTEGER, REAL and BOOLEAN.
- Equality and simple comparison filters in WHERE.
- Persistent database files, one JSON file per table.
- Clear errors for unsupported syntax.

## Requirements
- Rust stable (1.75+ recommended)

## Run
```sh
cargo run
```

Use a custom data directory:
```sh
ATDEV_DB_PATH=./data cargo run
```

Example:
```sql
CREATE TABLE users (id INTEGER, name TEXT, active BOOLEAN);
INSERT INTO users VALUES (1, 'Cris', true);
SELECT * FROM users;
UPDATE users SET active = false WHERE id = 1;
DELETE FROM users WHERE id = 1;
``

End the shell with `.exit`.

## Architecture
- `parser`: SQL tokenizer.
- `engine`: statement execution.
- `storage`: JSON-backed table persistence.

The storage format is an alpha implementation and may change before 1.0. Back up your data before upgrading.

## Roadmap
- Transactions and write-ahead logging.
- B-tree indexes and query planning.
- Constraints, joins, aggregates and prepared statements.
- Client/server protocol and AtDev Cloud adapter.
- Crash recovery and concurrent access.

## License
MIT
