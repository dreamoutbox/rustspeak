# PLAN: rustspeak (phase-1 product scope)

Text + voice communication app (TeamSpeak/Discord-like) in Rust, browser client first.

## Decisions (from grill-me)

| Topic | Decision |
|---|---|
| Server model | TeamSpeak-style: one deployment = one server; channels live directly under it |
| Roles | Fixed enum `Owner / Admin / Member`, global (no custom roles, no permission bitflags) |
| Voice | SFU in Rust, one voice room per voice channel |
| WebRTC lib | crates.io `webrtc` (latest stable 0.21.x; pin exact version in `[workspace.dependencies]`) |
| Text transport | WebSocket (axum) for realtime + REST for CRUD/history; persisted in Postgres |
| Auth | Argon2 password hash + JWT access token + rotating refresh token |
| TLS | Native `rustls` in the Rust server (HTTPS/WSS); media is DTLS-SRTP; dev certs via mkcert |
| Stack | axum, tokio, sqlx, Postgres 18 (Docker Compose), Vue 3 + TS + Vite + Bootstrap 5 in `web/` |
| Out of scope | file uploads, video/screen share, E2E encryption, edit/reactions, push, email verification, password reset |

## Workspace layout (target)

```
Cargo.toml            # [workspace] only
crates/rs-common/     # shared ids, error types, config, JWT claims types
crates/rs-user/       # users, auth, roles
crates/rs-text/       # text channels, messages, DMs
crates/rs-voice/      # voice channels, signaling, SFU
crates/rs-server/     # bin: axum wiring, TLS, WS gateway, tracing init
web/                  # Vue 3 + TS + Vite + Bootstrap 5
migrations/           # sqlx migrations
docker-compose.yml, scripts/
```

Each lib crate: `thiserror` errors in `error.rs`; `rs-server` uses `anyhow` at boundaries. One type per file; `mod.rs` wiring only. Tests in `*_test.rs`, run with `cargo nextest run --test-threads 1 --fail-fast`.

## Phases

### Phase 1: Workspace scaffold [STATUS: DONE]
- Goal: Replace the single-package `Cargo.toml` with a workspace containing the five crates, all building.
- Tasks:
  - [x] Root `Cargo.toml` as workspace (edition 2024, shared `[workspace.dependencies]`)
  - [x] Create `rs-common`, `rs-user`, `rs-text`, `rs-voice` (lib) and `rs-server` (bin); move `src/main.rs` into `rs-server`
  - [x] `rs-server` main: `tracing_subscriber` with `EnvFilter`, `anyhow::Result`
  - [x] `.gitignore`, `rustfmt`/clippy baseline
- Done when: `cargo build`, `cargo clippy`, and `cargo nextest run --test-threads 1 --fail-fast` pass.

### Phase 2: Config, Postgres, migrations [STATUS: DONE]
- Goal: Server boots with typed config and a migrated Postgres 18.
- Tasks:
  - [x] `docker-compose.yml` with Postgres 18 (volume at `/var/lib/postgresql`), healthcheck via a script file in `scripts/`
  - [x] Config struct in `rs-common` (env-driven: DB URL, bind addr, JWT secret, TLS paths, ICE UDP port range)
  - [x] sqlx pool + `migrations/` runner at startup
  - [x] Test helper to get an isolated test DB
- Done when: `docker compose up -d` then server starts, migrates, and a DB-backed smoke test passes.

### Phase 3: User system – register/login [STATUS: DONE]
- Goal: Users can register and log in, receiving JWT access + refresh tokens.
- Tasks:
  - [x] `users` and `refresh_tokens` tables
  - [x] `rs-user`: Argon2 hashing, username/password validation, repository, service
  - [x] Access JWT (short TTL), refresh token rotation with revocation (store hashed)
  - [x] REST: `POST /api/auth/register`, `/login`, `/refresh`, `/logout`, `GET /api/users/me`
  - [x] Auth extractor in axum (validates access JWT)
- Done when: unit + integration tests cover register, duplicate user, bad password, refresh rotation, reuse of a revoked refresh token being rejected.

### Phase 4: Roles and authorization [STATUS: DONE]
- Goal: Global `Owner/Admin/Member` roles enforced on endpoints.
- Tasks:
  - [x] `Role` enum (in `rs-common` or `rs-user`), column on `users`
  - [x] First registered user becomes `Owner` (race-safe); later users `Member`
  - [x] `require_role` extractor/guard
  - [x] Admin endpoints: list users, change role (Owner only can create Admins; Owner role not transferable in phase 1), disable user
- Done when: tests show Member is rejected from admin routes, Admin cannot promote to Owner, first-user-owner rule holds under concurrent registration.

### Phase 5: TLS [STATUS: DONE]
- Goal: Server serves HTTPS/WSS natively through rustls.
- Tasks:
  - [x] `axum-server` (or equivalent) with rustls, cert/key paths from config
  - [x] `scripts/dev-cert.sh` using mkcert; document in README
  - [x] Refuse to start without TLS unless an explicit `--insecure-dev` flag/env is set
- Done when: `curl --cacert` against `https://localhost:<port>/api/health` succeeds and plain HTTP is not served.

### Phase 6: Text channels and messages (REST) [STATUS: DONE]
- Goal: Admins manage text channels; members post and read persisted messages.
- Tasks:
  - [x] `text_channels`, `messages` tables (ULID/UUIDv7 ids for ordering)
  - [x] `rs-text`: channel CRUD (Admin+), post message (length limit), history with cursor pagination
  - [x] REST routes under `/api/channels`
  - [x] Seed a default `#general` channel on first boot
- Done when: integration tests cover permissions, pagination order, and limits.

### Phase 7: WebSocket gateway and realtime text [STATUS: DONE]
- Goal: Connected clients receive new messages live.
- Tasks:
  - [x] `/ws` endpoint authenticating via access token (query/first message; never log it)
  - [x] Connection registry + broadcast hub in `rs-server` (typed event enum, serde JSON)
  - [x] Publish `message.created` and channel create/delete events
  - [x] Heartbeat/ping, reconnect-friendly (client resumes via REST history)
  - [x] Basic presence (online/offline)
- Done when: a two-client integration test shows one client's post arriving on the other; unauthenticated WS rejected.

### Phase 8: Direct messages [STATUS: DONE]
- Goal: 1:1 persisted DMs delivered in realtime.
- Tasks:
  - [x] `dm_conversations` (unique user pair) + `dm_messages` (or reuse `messages` with a conversation kind; decide in this phase)
  - [x] REST: open/list conversations, history, send
  - [x] WS delivery only to the two participants
- Done when: tests show a third user can neither read nor receive a DM; pair uniqueness is enforced.

### Phase 9: Voice channels and signaling [STATUS: TODO]
- Goal: Clients can join/leave a voice channel and exchange SDP/ICE via WS, with no media yet.
- Tasks:
  - [ ] `voice_channels` table, CRUD (Admin+)
  - [ ] Signaling messages on the WS gateway: `voice.join`, `voice.leave`, `voice.offer/answer`, `voice.ice`
  - [ ] Room membership state (in-memory) + `voice.participants` events; a user is in at most one room
  - [ ] Cleanup on WS disconnect
- Done when: tests verify join/leave/disconnect membership events and the one-room rule.

### Phase 10: SFU audio relay [STATUS: TODO]
- Goal: Audio from one participant is forwarded to others in the same voice channel.
- Tasks:
  - [ ] `webrtc` crate: one `RTCPeerConnection` per participant, Opus audio only
  - [ ] Receive track from publisher, fan out via local tracks to subscribers (excluding self)
  - [ ] Renegotiation when participants join/leave
  - [ ] ICE config: fixed UDP port range from config, optional STUN; document firewall/NAT needs
  - [ ] Server-side tests with two `webrtc` test clients exchanging RTP
- Done when: automated test sees RTP from client A at client B and not echoed to A; manual two-browser check passes (Phase 14).

### Phase 11: Frontend scaffold and auth UI [STATUS: TODO]
- Goal: Vue app with login/register and session handling.
- Tasks:
  - [ ] `web/` with Vite + Vue 3 + TS + Bootstrap 5, Pinia, Vue Router
  - [ ] API client with access token + refresh-on-401
  - [ ] Login/register pages, route guards, admin-only nav by role
  - [ ] Dev proxy to the HTTPS backend; build output served by `rs-server` (static files)
- Done when: `npm run build` and type-check pass; manual login works against the local server.

### Phase 12: Frontend text and DM UI [STATUS: TODO]
- Goal: Usable chat in the browser.
- Tasks:
  - [ ] Channel list, message list with infinite scroll up, composer
  - [ ] WS client with reconnect and gap fill from REST history
  - [ ] DM list/conversation view, online indicators
  - [ ] Admin panel: manage channels and user roles
- Done when: two browsers chat in a channel and by DM in real time.

### Phase 13: Frontend voice UI [STATUS: TODO]
- Goal: Join a voice channel from the browser and talk.
- Tasks:
  - [ ] Voice channel list with participants
  - [ ] `getUserMedia`, RTCPeerConnection, signaling via WS, audio playback elements
  - [ ] Mute/deafen, leave, speaking indicator (client-side level detection)
  - [ ] Clear error states (mic denied, ICE failed)
- Done when: two browsers hear each other over the local SFU via HTTPS.

### Phase 14: Hardening and docs [STATUS: TODO]
- Goal: Phase-1 release candidate.
- Tasks:
  - [ ] Rate limiting on auth and message endpoints
  - [ ] Input validation/size limits, CORS/CSP/security headers
  - [ ] Dockerfile + compose profile for the full stack (no inline scripts; entrypoints in `scripts/`)
  - [ ] README: setup, certs, ports (TCP 443/UDP range), architecture diagram
  - [ ] End-to-end smoke checklist
- Done when: fresh clone to a running app follows the README; full test suite is green.

## Unknowns, risks, assumptions

- Assumption: single server means roles are global. Your two role answers overlapped (per-server roles vs global), so I applied fixed enum roles at global scope; tell me if you want something else.
- Risk: the `webrtc` crate API is heavy and has had breaking changes across 0.x releases. Pin the exact version; keep it behind an `rs-voice` interface so the SFU logic stays testable. Phase 10 starts with a small spike (one publisher, one subscriber).
- Risk: browsers need a secure context for `getUserMedia`; the Phase 5 dev certs must be trusted by the dev browser (mkcert handles this). WSL: the Windows browser must trust the cert too.
- Risk: voice over WSL/Docker/NAT needs a fixed UDP range and correct advertised host IP; configured in Phase 2, verified in Phase 10.
- Assumption: Opus-only audio, no recording, no E2E encryption; media is encrypted hop-by-hop (DTLS-SRTP) to the SFU, which can decrypt it.
- Assumption: refresh tokens are stored client-side (not cookies); XSS exposure is accepted for phase 1.
- Assumption: DM storage design (separate table vs shared `messages`) is decided in Phase 8.

