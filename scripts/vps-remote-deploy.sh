#!/usr/bin/env bash
set -euo pipefail

VPS_IP="${1:?usage: vps-remote-deploy.sh <vps-ip>}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

if [ -f rustspeak-image.tar.gz ]; then
  echo "Loading Docker image..."
  gunzip -c rustspeak-image.tar.gz | docker load
  rm -f rustspeak-image.tar.gz
fi

if [ ! -f .env ]; then
  cp .env.example .env
  echo "Created .env from .env.example"
fi

if grep -q '^PUBLIC_HOST=' .env; then
  sed -i "s|^PUBLIC_HOST=.*|PUBLIC_HOST=${VPS_IP}.sslip.io|" .env
else
  printf '\nPUBLIC_HOST=%s.sslip.io\n' "$VPS_IP" >> .env
fi

if grep -q '^JWT_SECRET=dev-only-change-me' .env; then
  NEW_SECRET="$(openssl rand -hex 32)"
  sed -i "s|^JWT_SECRET=.*|JWT_SECRET=${NEW_SECRET}|" .env
fi

echo "Starting RustSpeak stack on https://${VPS_IP}.sslip.io ..."
SKIP_BUILD=1 ./setup.sh

echo "Done. Open https://${VPS_IP}.sslip.io"
