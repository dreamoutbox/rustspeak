#!/bin/sh
set -eu

CERT_DIR="${1:-certs}"

if ! command -v openssl >/dev/null 2>&1; then
    echo "error: openssl is not installed." >&2
    exit 1
fi

mkdir -p "$CERT_DIR"
openssl req -x509 -newkey rsa:2048 -sha256 -days 365 -nodes \
    -keyout "$CERT_DIR/localhost-key.pem" \
    -out "$CERT_DIR/localhost.pem" \
    -subj "/CN=localhost" \
    -addext "subjectAltName=DNS:localhost,IP:127.0.0.1,IP:::1"

echo "Generated self-signed dev certs in $CERT_DIR/localhost.pem and $CERT_DIR/localhost-key.pem"
