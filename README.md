# rustspeak

Text and voice communication app in Rust.

## Local Development Setup

### 1. Start PostgreSQL 18

```sh
docker compose up -d
```

### 2. Generate Development TLS Certificates

Install [mkcert](https://github.com/FiloSottile/mkcert) and run:

```sh
mkcert -install
./scripts/dev-cert.sh
```

This creates `certs/localhost.pem` and `certs/localhost-key.pem`. On WSL, ensure your Windows browser also trusts the mkcert root CA (`mkcert -CAROOT`).

### 3. Run the Server

Copy `.env.example` to `.env` and start the server:

```sh
cp .env.example .env
cargo run -p rs-server
```

To run without TLS in local development only, pass `--insecure-dev` or set `INSECURE_DEV=true`.
