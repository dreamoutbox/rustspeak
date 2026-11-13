#!/bin/sh
set -eu
pg_isready -U "${POSTGRES_USER:-rustspeak}" -d "${POSTGRES_DB:-rustspeak}"

