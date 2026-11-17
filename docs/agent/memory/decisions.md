# Decisions

Dated decisions and why. Read before work, write after work. Wrong entries get deleted, not appended to.

| Date | Decision | Why |
|------|----------|-----|
| 2026-11-12 | Standard radios only; P0 to P3 first; Windows then Android; name Relay; per-folder grants + audit log (approved 2026-11-12) | Owner's approved points 1 to 4, 7 in `docs/design/00-overview.md`. |
| 2026-11-17 | Rust core + Tauri app, chosen by owner on a decision card; one shared core, no bridge | One core serves Windows and Android with no bridge to fail. Rust has mature QUIC and crypto libraries. |
| 2026-11-17 | Friendly-site trust model; internet optional, never required | Owner's answers: members are people the host knows; internet is an optional extra. |
| 2026-11-17 | Test hardware: 2 Windows PCs + 1 phone | Owner's answer on test hardware. |
| 2026-11-17 | UI: very dark or Windows-like with an Apple-style finish, no default purple | Owner's answer on the look. |
| 2026-11-17 | Agents work in the cloud and may push to GitHub branches; nothing touches the owner's computer without asking | Owner's answer on agent behavior. |
