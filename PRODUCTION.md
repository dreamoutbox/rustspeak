# Production VPS Deployment Guide

This guide explains how to deploy RustSpeak on a Linux VPS so friends can join via a browser-trusted HTTPS/WSS URL with zero manual certificate setup, while WebRTC voice media uses DTLS-SRTP over UDP.

## 1. Prerequisites

- Linux VPS with a public IPv4 address (e.g. `203.0.113.10`)
- Docker Engine and Docker Compose v2 (`docker compose version`)

## 2. Open Firewall / Cloud Security Group Ports

Open these ports on your VPS firewall (`ufw`, `iptables`, or cloud security group):

| Port / Protocol | Required | Purpose |
| --- | --- | --- |
| `80/tcp` | Yes | HTTP ACME challenge + automatic redirect to HTTPS (Caddy) |
| `443/tcp` | Yes | HTTPS REST API, web UI, and WSS WebSocket signaling (Caddy) |
| `40000-40100/udp` | Yes | WebRTC SFU DTLS-SRTP Opus voice media (`app`) |
| `5432/tcp`, `8080/tcp` | **No** | Keep Postgres and Adminer closed to the public internet (SSH tunnel if needed) |

Example with `ufw`:

```sh
sudo ufw allow 22/tcp
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw allow 40000:40100/udp
sudo ufw enable
```

## 3. Choose a Hostname (`sslip.io` if You Have No Domain)

Browsers require a trusted `https://` origin to allow WebSocket TLS (`wss://`) and microphone access (`getUserMedia`).

- **If you own a domain**: Point an `A` record (e.g. `chat.example.com`) to your VPS IP.
- **If you only have a VPS IP (no domain)**: Use **`sslip.io`**.
  - `sslip.io` is a free wildcard DNS service that automatically resolves `<VPS_IP>.sslip.io` to `<VPS_IP>`.
  - Example: if your VPS IP is `203.0.113.10`, use `203.0.113.10.sslip.io`.
  - Caddy uses that hostname to automatically obtain and renew a real, browser-trusted **Let's Encrypt** TLS certificate — your friends just open `https://203.0.113.10.sslip.io` with no self-signed cert warnings.

## 4. Configure `.env`

Copy `.env.example` to `.env` and edit the production values:

```sh
cp .env.example .env
```

Set at minimum:

```env
# Replace with your domain or <VPS_IP>.sslip.io
PUBLIC_HOST=203.0.113.10.sslip.io

# Generate a strong secret (32+ random bytes), e.g.: openssl rand -hex 32
JWT_SECRET=replace-with-at-least-32-random-bytes

# Strong database password
POSTGRES_PASSWORD=replace-with-strong-password
DATABASE_URL=postgres://rustspeak:replace-with-strong-password@postgres:5432/rustspeak

# WebRTC SFU + STUN
ICE_BIND_IP=0.0.0.0
STUN_URL=stun:stun.l.google.com:19302
ICE_UDP_PORT_MIN=40000
ICE_UDP_PORT_MAX=40100
```

## 5. Start the Stack

```sh
./setup.sh
```

This builds the Rust server + Vue frontend image, starts PostgreSQL 18, runs migrations automatically on `app` startup, and starts Caddy on ports `80`/`443`.

## 6. First User & Smoke Check

1. Open `https://<PUBLIC_HOST>` in a browser.
2. Verify the browser shows a valid TLS lock (issued by Let's Encrypt) with no certificate warning.
3. Register the first user — they are automatically assigned the `owner` role.
4. Open a second browser/incognito window, register another user (`member`), and test:
   - Realtime text messages in `#general`
   - Direct messages
   - Joining a voice channel (users without a microphone join in listen-only mode)

## 7. Logs & Troubleshooting

- View Caddy TLS/ACME logs: `docker compose logs -f caddy`
- View Rust server logs: `docker compose logs -f app`
- Verify health: `curl -fsS https://<PUBLIC_HOST>/api/health`
- If voice stays in `connecting` or fails ICE, verify `40000-40100/udp` is open in both the OS firewall and your VPS provider's cloud security group.

