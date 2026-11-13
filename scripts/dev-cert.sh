#!/bin/sh
set -eu

CERT_DIR="${1:-certs}"

if ! command -v mkcert >/dev/null 2>&1; then
    echo "error: mkcert is not installed. Install mkcert and run 'mkcert -install' first." >&2
    exit 1
fi

mkdir -p "$CERT_DIR"
mkcert -cert-file "$CERT_DIR/localhost.pem" -key-file "$CERT_DIR/localhost-key.pem" localhost 127.0.0.1 ::1
echo "Generated dev certs in $CERT_DIR/localhost.pem and $CERT_DIR/localhost-key.pem"
