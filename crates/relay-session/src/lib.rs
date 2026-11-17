//! Relay `session` module (error prefix `RLY-SS`). One encrypted, authenticated connection per peer; reconnect and resume.
//!
//! Boundaries and rules: see `docs/design/01-architecture.md` and
//! `docs/agent/modules/session.md`. Interfaces only for now; no transport yet.

use relay_common::{DeviceId, ErrorCode, Module, RelayError};

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
    /// The session is not established, so it cannot carry messages.
    #[error("{code} session is not established (state {state:?})", code = ErrorCode::new(MODULE, 2))]
    NotEstablished {
        /// The state the session was in.
        state: SessionState,
    },
    /// The session was already closed.
    #[error("{code} session is closed", code = ErrorCode::new(MODULE, 3))]
    Closed,
}

impl RelayError for Error {
    fn code(&self) -> ErrorCode {
        match self {
            Error::NotImplemented { .. } => ErrorCode::new(MODULE, 1),
            Error::NotEstablished { .. } => ErrorCode::new(MODULE, 2),
            Error::Closed => ErrorCode::new(MODULE, 3),
        }
    }
}

/// Identifies one session. A session outlives any single connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionId(pub u64);

/// Where a session is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionState {
    /// Opening a connection to the peer.
    Connecting,
    /// Exchanging keys and certificates.
    Authenticating,
    /// Ready to carry messages.
    Established,
    /// Connection lost; retrying on another path.
    Reconnecting,
    /// Finished; cannot be reused.
    Closed,
}

/// The two stream types on a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StreamKind {
    /// One stream for small control messages.
    Control,
    /// Many streams for file chunks.
    Data,
}

/// One session with one peer.
pub trait Session {
    /// The peer on the other end.
    fn peer(&self) -> DeviceId;
    /// The current state.
    fn state(&self) -> SessionState;
    /// Sends a message on the control stream.
    fn send_control(&mut self, bytes: &[u8]) -> Result<(), Error>;
    /// Closes the session.
    fn close(&mut self) -> Result<(), Error>;
}

/// Placeholder session: stays in `Connecting` and refuses to send.
#[derive(Debug, Clone, Copy)]
pub struct UnbuiltSession {
    peer: DeviceId,
}

impl UnbuiltSession {
    /// Creates a placeholder session for `peer`.
    pub fn new(peer: DeviceId) -> Self {
        Self { peer }
    }
}

impl Session for UnbuiltSession {
    fn peer(&self) -> DeviceId {
        self.peer
    }
    fn state(&self) -> SessionState {
        SessionState::Connecting
    }
    fn send_control(&mut self, _bytes: &[u8]) -> Result<(), Error> {
        Err(Error::NotImplemented {
            what: "session send_control",
        })
    }
    fn close(&mut self) -> Result<(), Error> {
        Err(Error::NotImplemented {
            what: "session close",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_error_has_matching_code_and_display_prefix() {
        let all = [
            (Error::NotImplemented { what: "x" }, "RLY-SS-001"),
            (
                Error::NotEstablished {
                    state: SessionState::Connecting,
                },
                "RLY-SS-002",
            ),
            (Error::Closed, "RLY-SS-003"),
        ];
        for (e, code) in all {
            assert_eq!(e.code().to_string(), code);
            assert!(e.to_string().starts_with(code), "{e}");
        }
    }

    #[test]
    fn unbuilt_session_stubs() {
        let mut s = UnbuiltSession::new(DeviceId([7; 32]));
        assert_eq!(s.peer(), DeviceId([7; 32]));
        assert_eq!(s.state(), SessionState::Connecting);
        assert!(matches!(
            s.send_control(b"x"),
            Err(Error::NotImplemented { .. })
        ));
        assert!(matches!(s.close(), Err(Error::NotImplemented { .. })));
    }
}
