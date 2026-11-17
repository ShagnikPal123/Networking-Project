# Module: identity (ID)

Crate: `crates/relay-identity` · Error prefix: `RLY-ID-<N>`
First built: P0 Foundation (device keys and fingerprints).

## Purpose

Device keys, site keys, certificates, and chain checks.

## Owns

- Device keys and site keys
- Certificates and chain checks

## Must never

- Touch the network or disk paths it was not given

## Depends on

Nothing. Identity sits at the base.

## Module checklist

- [ ] Host and member certificates are checked back to the owner's site key (owner, then host, then member).
- [ ] A certificate that fails a chain check returns an `RLY-ID-<N>` error and is not used.
- [ ] Any change to key or certificate handling runs the advanced checklist.

## Memory

### Notes

None yet.
