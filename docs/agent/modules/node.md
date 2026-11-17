# Module: node (ND)

Crate: `crates/relay-node` · Error prefix: `RLY-ND-<N>`
First built: P0 Foundation (module health and error codes).

## Purpose

Starts modules, routes events, and reports health.

## Owns

- Starting modules
- The event bus (`PeerFound`, `PathDown`, `JoinRequested`, `ChunkVerified`, `HostChanged`...)
- Health reporting to the UI

## Must never

- Contain protocol logic

## Depends on

- transfer (TX), share (SH), membership (MB), audit (AU), discovery (DS)

## Module checklist

- [ ] A module that reports `Degraded` or `Down` does not take down modules that do not depend on it. Test it.
- [ ] Events go up through the node bus. No module calls a module above it.
- [ ] The UI calls modules only through node.

## Memory

### Notes

None yet.
