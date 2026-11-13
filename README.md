# rustspeak

Text and voice communication app in Rust.

## Local Development Setup

### 1. Start PostgreSQL 18

```sh
docker compose up -d
```

### 2. Generate Development TLS Certificates

Run the OpenSSL script to generate a self-signed certificate for `localhost` and `127.0.0.1`:

```sh
./scripts/dev-cert.sh
```

This creates `certs/localhost.pem` and `certs/localhost-key.pem`. Because it is self-signed, your browser will ask you to accept the certificate on first visit (or you can trust `certs/localhost.pem` in your OS/browser certificate store).

### 3. Run the Server

Copy `.env.example` to `.env` and start the server:

```sh
cp .env.example .env
cargo run -p rs-server
```

To run without TLS in local development only, pass `--insecure-dev` or set `INSECURE_DEV=true`.
