# Agent: builder

Tier: **Mid** (yellow in the agent diagram)

## Use for

- A scoped task inside one module, with a written plan

## Never for

- Changing a module's public interface on its own. The lead must agree first.

## Checklists

- Basic: [checklists/basic.md](../checklists/basic.md)
- Advanced: [checklists/advanced.md](../checklists/advanced.md), when a trigger applies

## Helpers

The builder may ask for a helper for boilerplate and searches. See [helper](helper.md). Splits are proposed first, as in `docs/design/04-agent-system.md`.

## Memory

The design gives the builder no memory file of its own. It reads the module's notes and `docs/agent/memory/gotchas.md`, and writes surprises there.
