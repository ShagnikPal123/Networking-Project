//! Relay `session` module (error prefix `RLY-SS`). One encrypted, authenticated connection per peer; reconnect and resume.
//!
//! Boundaries and rules: see `docs/design/01-architecture.md` and
//! `docs/agent/modules/session.md`.

use relay_common::{ErrorCode, Module, RelayError};

/// This crate's module tag, used in error codes and health reports.
pub const MODULE: Module = Module::Session;

/// Errors raised by the `session` module.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The feature exists in the design but is not built yet.
    #[error("{code} not implemented yet: {what}", code = ErrorCode::new(MODULE, 1))]
    NotImplemented {
        /// The missing feature.
        what: &'static str,
    },
}

impl RelayError for Error {
    fn code(&self) -> ErrorCode {
        match self {
            Error::NotImplemented { .. } => ErrorCode::new(MODULE, 1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_has_module_prefix() {
        let e = Error::NotImplemented { what: "x" };
        assert_eq!(e.code().to_string(), "RLY-SS-001");
        assert!(e.to_string().starts_with("RLY-SS-001"));
    }
}
