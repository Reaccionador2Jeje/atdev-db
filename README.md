# AtDev DB

An independent, open-source relational SQL engine by AtDev Dev. It is an experimental alpha with a deliberately small SQL dialect and JSON-backed table storage; it is **not yet a production database or PostgreSQL replacement**.

## Features

- Interactive SQL shell.
- `CREATE TABLE`, `INSERT`, `SELECT`, `UPDATE`, and `DELETE`.
- TEXT, INTEGER, REAL, and BOOLEAN values.
- Simple comparison filters.
- Persistent table files, one JSON file per table.
- Optional HTTP service for AtDev Cloud.
- Bearer-token protection for non-loopback HTTP deployments.
- Docker image and persistent Docker volume.

## Run the SQL shell

Requires Rust stable (1.75+ recommended):

```sh
cargo run
```

Set a custom data directory with `ATDEV_DB_PATH=./data`.

## Run locally as an HTTP service

For local development, loopback binding is the default and a token is optional:

```sh
ATDEV_DB_BIND=127.0.0.1:8788 cargo run -- --server
```

- `GET /health` is public.
- `POST /v1/query` accepts JSON: `{"database":"user_project","sql":"SELECT * FROM notes"}`.
- If `ATDEV_DB_TOKEN` is set, send `Authorization: Bearer <token>`.
- Binding to a non-loopback interface requires `ATDEV_DB_TOKEN`.
- HTTP request bodies are limited to 1 MiB and SQL statements to 10,000 bytes.

The API is intended to sit behind AtDev Cloud or a private reverse proxy. Do not expose the service publicly without HTTPS, a strong token, backups, and network-level protections.

## Docker Compose

```sh
cp .env.example .env
# Replace ATDEV_DB_TOKEN with a long random secret.
docker compose up -d --build
```

The container listens internally on port 8788; Compose publishes it only on host loopback at `127.0.0.1:8788`. Data persists in the `atdev-db-data` named volume.

To connect AtDev Cloud, configure its Worker secrets:

- `ATDEV_DB_URL`: HTTPS URL of a reverse proxy that reaches this service.
- `ATDEV_DB_TOKEN`: exactly the same secret as the service's environment variable.

Never commit the real token or expose the raw container port to the public internet.

## SQL example

```sql
CREATE TABLE users (id INTEGER, name TEXT, active BOOLEAN);
INSERT INTO users VALUES (1, 'Cris', true);
SELECT * FROM users;
UPDATE users SET active = false WHERE id = 1;
DELETE FROM users WHERE id = 1;
```

End the shell with `.exit`.

## Architecture and portability

- `parser`: SQL tokenizer.
- `engine`: statement execution.
- `storage`: JSON-backed table persistence.
- Docker/Podman-compatible OCI container.
- Data is stored under `ATDEV_DB_PATH`; copy the directory or volume while the service is stopped to back it up.

The JSON storage format is alpha and may change before 1.0. Back up data before upgrades. Concurrent access, transactions, WAL, indexes, joins, crash recovery, and PostgreSQL wire compatibility are not implemented yet.

## License

MIT
