# Configuration Notes

## `INSECURE_DEV`
- Controls whether `rs-server` serves plain HTTP/WS instead of terminating TLS itself (`TLS_CERT_PATH` / `TLS_KEY_PATH`).
- **Docker Compose (`INSECURE_DEV=true`)**: Safe because `app` only exposes port `8443` on the internal Docker network (not published to the host). Caddy handles public HTTP/HTTPS/WSS on ports `80` and `443`.
- **Direct Cargo run**: Set `INSECURE_DEV=true` (or pass `--insecure-dev`) only for local `127.0.0.1` testing without Caddy or certs.

## `STUN_URL`
- Optional STUN server URL (default in Compose: `stun:stun.l.google.com:19302`).
- Used by both the SFU (`rs-voice`) and browser `RTCPeerConnection` (fetched from `GET /api/voice/config`) to discover public IP:port mappings behind NAT during WebRTC ICE negotiation.
- STUN only assists connection discovery; actual voice media (DTLS-SRTP) flows directly between the browser and the SFU over UDP `40000-40100`.

## `ICE_BIND_IP`
- The network interface IP that `rs-voice` binds its WebRTC UDP media sockets to.
- Defaults to `127.0.0.1` in `rs-common` (for local single-machine runs) and `0.0.0.0` in `docker-compose.yml` (so the container can accept UDP media packets from LAN/VPS clients on `40000-40100/udp`).

## Do we still need `./certs`?
- **No** when running via Docker Compose (`./setup.sh`). Caddy handles TLS automatically:
  - Localhost (`PUBLIC_HOST=:80`): serves `http://localhost` (browsers treat `localhost` as a Secure Context for microphone access).
  - VPS (`PUBLIC_HOST=<VPS_IP>.sslip.io` or your domain): Caddy automatically obtains and renews a browser-trusted Let's Encrypt certificate on `:443`, so friends do not need to trust a self-signed cert.
- **Yes** only if you run `cargo run -p rs-server` directly with `INSECURE_DEV=false` and no Caddy reverse proxy in front (generate with `./scripts/dev-cert.sh`).

