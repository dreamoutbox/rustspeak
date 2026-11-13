# Postmortem: Voice Renegotiation Failure on Peer Rejoin

**Date:** 2026-11-13
**Severity:** Medium
**Status:** Resolved

## Summary
When a user disconnected from a voice channel and rejoined while another participant remained in the room, the remaining participant's browser failed WebRTC renegotiation with `"Failed to negotiate voice renegotiation."`. The SFU was assigning deterministic `rustspeak-<publisher_id>` track and stream IDs to outbound tracks, causing a duplicate track/MSID collision in the subscriber's `RTCPeerConnection`.

## Impact
Remaining participants in a voice channel could not renegotiate audio from a rejoining peer and entered a `failed` voice connection state until they left and rejoined the channel.

## Root Cause
- In `crates/rs-voice/src/sfu.rs` (`attach_track_to_subscriber`), every `TrackLocalStaticRTP` was created with static IDs derived solely from `publisher_id` (`rustspeak-<publisher_id>` and `audio-<publisher_id>-<subscriber_id>`).
- When `user2` disconnected (`remove_peer`), `user2`'s entry was removed from `user1`'s `outbound_tracks` map, but the browser's `RTCPeerConnection` still retained the prior media transceiver/MSID from the first session.
- When `user2` rejoined, the SFU added a new `TrackLocalStaticRTP` with the identical track/stream IDs and sent a renegotiation offer. The browser rejected `setRemoteDescription` due to the duplicate track/MSID identifier.
- Additionally, `web/src/stores/voice.ts` caught the signaling error without logging the underlying exception to `console.error`, hiding the exact browser `DOMException` in DevTools.

## Detection
Manual multi-user testing (`user1` joins VC -> `user2` joins VC -> `user2` disconnects -> `user2` rejoins -> `user1` hits `"Failed to negotiate voice renegotiation."`), recorded in `TODO.md`.

## Resolution
- Updated `crates/rs-voice/src/sfu.rs` (`attach_track_to_subscriber`) to generate a fresh `Uuid::now_v7()` per `OutboundTrack` instance for its track ID, label, and stream ID.
- Updated `web/src/stores/voice.ts` (`handleServerEvent`) to log `console.debug` and `console.error` diagnostics for `voice.offer`, `voice.answer`, and `voice.ice`.
- Verified `npm run type-check`, `npm run build`, `cargo clippy`, and `cargo nextest run --workspace --test-threads 1 --fail-fast`.

## Lessons Learned / Action Items
- [ ] Add an automated SFU integration test in `crates/rs-voice/src/voice_test.rs` covering peer A + peer B join, peer B disconnect, peer B rejoin, and verifying peer A's renegotiation offer/answer succeeds.
- [ ] Also call `remove_track` + renegotiate remaining peers on `remove_peer` in `crates/rs-voice/src/sfu.rs` so stale inactive transceivers do not accumulate across many joins/leaves.

