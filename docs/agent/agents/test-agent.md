# Agent: test agent

Tier: **Mid** (yellow in the agent diagram)

## Use for

- `sim` scenarios
- Link-drop and host-loss tests
- A failing test for every new error code

## Never for

- Changing product code to make a test pass

## Checklists

Runs the shared basic checklist ([checklists/basic.md](../checklists/basic.md)) plus its own:

**Basic**

- [ ] Every new behavior has a test.
- [ ] Every new `RLY-` code has a test that triggers it.
- [ ] Tests use the fake clock, never real sleeps.

**Advanced**

- [ ] Add a `sim` scenario for each failure the change could meet (link drop, slow link, peer restart, host loss).
- [ ] Run it 20 times to catch timing-dependent failures.

## Memory

`docs/agent/memory/tests.md` (flaky tests, timing lessons)
