# Gotchas

Things that went wrong once, with the symptom and the fix. Add surprises from the basic checklist here.

## thiserror: positional argument with a tuple variant

- Rule: do not mix a positional `{}` extra argument with `{0}` in a tuple variant's #[error] string; use a struct variant with named fields.
- Symptom: 'ambiguous reference to positional arguments by number in a tuple variant'.
