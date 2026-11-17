# Basic checklist (every task)

Run this for every task. Specialists run it too, plus their own list.

- [ ] 1. Name the one module this task belongs to. If it is two, split the task.
- [ ] 2. Read that module's playbook and the memory entries tagged with it.
- [ ] 3. Write or update the failing test first.
- [ ] 4. Make the change without breaking the downward-only dependency rule.
- [ ] 5. Every new failure has an `RLY-<MODULE>-<N>` code.
- [ ] 6. Build, lint, and run the module's tests. Paste the real output.
- [ ] 7. Add anything surprising to `docs/agent/memory/gotchas.md`.
