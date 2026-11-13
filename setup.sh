#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

if [ ! -f .env ]; then
  cp .env.example .env
  echo "Created .env from .env.example"
else
  echo ".env already exists; keeping existing file"
fi

if [ "${SKIP_BUILD:-0}" = "1" ]; then
  docker compose up -d --no-build "$@"
else
  docker compose up -d --build "$@"
fi
