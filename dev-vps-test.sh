#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 1 ]; then
  echo "Usage: $0 <user@vps-ip> [remote-dir]" >&2
  exit 1
fi

TARGET="$1"
REMOTE_DIR="${2:-rustspeak}"
VPS_IP="${TARGET#*@}"

if [ -z "$VPS_IP" ] || [ "$VPS_IP" = "$TARGET" ]; then
  echo "Target must be in the form user@<vps-ip> (e.g. root@203.0.113.10)" >&2
  exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

IMAGE_TAR="$(mktemp /tmp/rustspeak-image.XXXXXX.tar.gz)"
cleanup() {
  rm -f "$IMAGE_TAR"
}
trap cleanup EXIT

echo "[1/5] Building Docker image locally..."
./scripts/build-docker-image.sh

echo "[2/5] Saving and compressing image..."
docker save rustspeak:latest | gzip > "$IMAGE_TAR"

echo "[3/5] Uploading repository to ${TARGET}:~/${REMOTE_DIR}..."
ssh "$TARGET" "mkdir -p ~/${REMOTE_DIR}"
rsync -avz --delete \
  --exclude '.git' \
  --exclude 'target' \
  --exclude 'data' \
  --exclude 'node_modules' \
  --exclude 'web/node_modules' \
  --exclude 'web/dist' \
  --exclude 'certs' \
  --exclude '.env' \
  --exclude '*.tar' \
  --exclude '*.tar.gz' \
  ./ "${TARGET}:~/${REMOTE_DIR}/"

echo "[4/5] Uploading Docker image to ${TARGET}:~/${REMOTE_DIR}/rustspeak-image.tar.gz..."
rsync -avz --progress "$IMAGE_TAR" "${TARGET}:~/${REMOTE_DIR}/rustspeak-image.tar.gz"

echo "[5/5] Configuring .env and starting stack on VPS..."
ssh "$TARGET" "bash ~/${REMOTE_DIR}/scripts/vps-remote-deploy.sh ${VPS_IP}"

