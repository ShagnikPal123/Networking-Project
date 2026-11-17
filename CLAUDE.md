# Relay: agent entry point

Relay is a portable, self-healing local network with file transfer for one site (a home, an office, an event, or a field team). It keeps working with no internet. If a link drops, traffic moves to another link. If the host drops, an owner-approved device takes over.

The first target is P0 to P3 (see `docs/design/03-phases.md`). The design lives in `docs/design/`. Start with `docs/design/00-overview.md`.

## Hard rules

- Work runs in the cloud. Never touch the owner's computer or its settings unless the owner asks first.
- Agents may push to GitHub branches on their own.
- Propose a split before spawning sub-agents. Wait for a yes.
- Use the cheapest agent tier that can do the task safely.
- Modules depend downward only. See `docs/design/01-architecture.md`.
- Every failure has an `RLY-<MODULE>-<N>` error code.
- Update `NEXT-UPDATES.md` in the same PR as the work.

## Checklists

- Every task: [docs/agent/checklists/basic.md](docs/agent/checklists/basic.md)
- Risky tasks: [docs/agent/checklists/advanced.md](docs/agent/checklists/advanced.md)

## Build and verify

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Router: module to playbook

| Module | Error prefix | Crate | Playbook |
|--------|--------------|-------|----------|
| identity | `RLY-ID-` | `crates/relay-identity` | [docs/agent/modules/identity.md](docs/agent/modules/identity.md) |
| proto | `RLY-PR-` | `crates/relay-proto` | [docs/agent/modules/proto.md](docs/agent/modules/proto.md) |
| store | `RLY-ST-` | `crates/relay-store` | [docs/agent/modules/store.md](docs/agent/modules/store.md) |
| discovery | `RLY-DS-` | `crates/relay-discovery` | [docs/agent/modules/discovery.md](docs/agent/modules/discovery.md) |
| link | `RLY-LK-` | `crates/relay-link` | [docs/agent/modules/link.md](docs/agent/modules/link.md) |
| session | `RLY-SS-` | `crates/relay-session` | [docs/agent/modules/session.md](docs/agent/modules/session.md) |
| membership | `RLY-MB-` | `crates/relay-membership` | [docs/agent/modules/membership.md](docs/agent/modules/membership.md) |
| transfer | `RLY-TX-` | `crates/relay-transfer` | [docs/agent/modules/transfer.md](docs/agent/modules/transfer.md) |
| share | `RLY-SH-` | `crates/relay-share` | [docs/agent/modules/share.md](docs/agent/modules/share.md) |
| audit | `RLY-AU-` | `crates/relay-audit` | [docs/agent/modules/audit.md](docs/agent/modules/audit.md) |
| node | `RLY-ND-` | `crates/relay-node` | [docs/agent/modules/node.md](docs/agent/modules/node.md) |

## Agents

| Agent | Tier | Playbook |
|-------|------|----------|
| Lead | Strongest | [docs/agent/agents/lead.md](docs/agent/agents/lead.md) |
| Idea agent | Strong | [docs/agent/agents/idea-agent.md](docs/agent/agents/idea-agent.md) |
| Security reviewer | Strong | [docs/agent/agents/security-reviewer.md](docs/agent/agents/security-reviewer.md) |
| Design reviewer | Strong | [docs/agent/agents/design-reviewer.md](docs/agent/agents/design-reviewer.md) |
| Builder | Mid | [docs/agent/agents/builder.md](docs/agent/agents/builder.md) |
| Test agent | Mid | [docs/agent/agents/test-agent.md](docs/agent/agents/test-agent.md) |
| Docs agent | Cheapest | [docs/agent/agents/docs-agent.md](docs/agent/agents/docs-agent.md) |
| Helper | Cheapest | [docs/agent/agents/helper.md](docs/agent/agents/helper.md) |

## Memory

Read before work, write after work. Save decisions with a date and a reason. Save gotchas with the symptom and the fix.

- Decisions: [docs/agent/memory/decisions.md](docs/agent/memory/decisions.md)
- Gotchas: [docs/agent/memory/gotchas.md](docs/agent/memory/gotchas.md)
