# Builder A memory

Scope: base crates. Current task scope: `relay-identity` only.

## 2026-11-17 — real Ed25519 in relay-identity

Decisions (all are calls a lead may overrule):

- Crypto library: `ed25519-dalek` 2, `verify_strict`, weak keys refused (`RLY-ID-006`).
- Device ID = SHA-256(`"relay/device-id/v1" | 0 | public key`). Site ID uses the tag
  `relay/site-id/v1`. SHA-256 was picked over BLAKE3 because `ed25519-dalek` already brings
  `sha2`, so the security-critical crate gains no extra hash dependency.
- Signed bytes are a fixed big-endian encoding with a kind+version tag, defined in
  `HostCertificate::signing_bytes` and `MemberCertificate::signing_bytes`. `proto` may choose a
  different wire encoding; signatures always cover these bytes.
- Expiry is exclusive: valid while `now < expiry`.
- `verify_chain` requires member epoch == host epoch. Stricter than "<=" on purpose, so a device
  that was host at an old epoch cannot have its old member certificates accepted again.
- Persistence: identity does no I/O. Callers save `DeviceKeyPair::secret_bytes()` via `store`
  and reload with `from_secret_bytes`.

Interface: existing function signatures were kept. Added `site_id`, `sign_host_certificate`,
`sign_member_certificate`, `verify_host_certificate`, `SiteKeyPair`, key pair methods, and error
codes 006 (bad public key) and 007 (random source). `RLY-ID-001` (not implemented) is now unused
but stays, since numbers are never reused.

Gotchas:

- Dependencies are declared in `crates/relay-identity/Cargo.toml`, not the workspace manifest,
  to avoid touching a shared file. `Cargo.lock` still changes; expect a trivial merge there.
- No size limits yet on `name` or `deputies`; the decoder in `proto` should cap them.
- Not yet covered: revocation lists, pre-signed deputy certificates (P2/P3).
