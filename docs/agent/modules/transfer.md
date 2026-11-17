# Module: transfer (TX)

Crate: `crates/relay-transfer` · Error prefix: `RLY-TX-<N>`
First built: P1 Transfer (offer and accept, chunking and hashing, parallel streams, resume from disk).

## Purpose

Manifests, chunks, verification, and resume.

## Owns

- Manifests
- Chunks, hashing, and verify
- Resume

## Must never

- Decide access rights

## Depends on

- session (SS), store (ST), proto (PR)

## Module checklist

- [ ] A corrupt chunk or file is never delivered silently. A hash mismatch returns an `RLY-TX-<N>` error.
- [ ] Kill the receiving app halfway, restart, and test that the transfer resumes and the hash matches.
- [ ] Progress events go up through node for the UI.

## Memory

### Notes

None yet.
