//! Relay `discovery` module (error prefix `RLY-DS`). Finding sites and peers on the LAN. Never decides trust.
//!
//! Boundaries and rules: see `docs/design/01-architecture.md` and
//! `docs/agent/modules/discovery.md`. Discovery only says "something is here"; it never grants
//! trust (protocol section 2).

use relay_common::{DeviceId, ErrorCode, Module, RelayError, SiteId};
use std::net::SocketAddr;

/// This crate's module tag, used in error codes and health reports.
pub const MODULE: Module = Module::Discovery;

/// How often a host sends its beacon, in milliseconds.
pub const BEACON_INTERVAL_MS: u64 = 2_000;

/// Errors raised by the `discovery` module.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The feature exists in the design but is not built yet.
    #[error("{code} not implemented yet: {what}", code = ErrorCode::new(MODULE, 1))]
    NotImplemented {
        /// The missing feature.
        what: &'static str,
    },
    /// `start` was called while discovery was already running.
    #[error("{code} discovery is already running", code = ErrorCode::new(MODULE, 2))]
    AlreadyStarted,
    /// An operation needed discovery to be running.
    #[error("{code} discovery is not running", code = ErrorCode::new(MODULE, 3))]
    NotStarted,
    /// The discovery sockets could not be opened.
    #[error("{code} could not open discovery socket: {reason}", code = ErrorCode::new(MODULE, 4))]
    BindFailed {
        /// Human-readable cause.
        reason: String,
    },
}

impl RelayError for Error {
    fn code(&self) -> ErrorCode {
        match self {
            Error::NotImplemented { .. } => ErrorCode::new(MODULE, 1),
            Error::AlreadyStarted => ErrorCode::new(MODULE, 2),
            Error::NotStarted => ErrorCode::new(MODULE, 3),
            Error::BindFailed { .. } => ErrorCode::new(MODULE, 4),
        }
    }
}

/// Version of the beacon layout.
pub const BEACON_VERSION: u8 = 1;

/// The content of a UDP broadcast beacon. It is signed by the sending device's key.
///
/// It deliberately leaves out who the host is and the epoch: those travel only inside
/// authenticated sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Beacon {
    /// Beacon layout version, [`BEACON_VERSION`].
    pub version: u8,
    /// The site the sender belongs to, or `None` before it has joined one.
    pub site: Option<SiteId>,
    /// The device sending the beacon.
    pub device: DeviceId,
    /// UDP port the sender accepts sessions on.
    pub port: u16,
}

/// How a peer was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiscoveryMethod {
    /// mDNS service `_relay._udp.local`.
    Mdns,
    /// Signed UDP broadcast beacon.
    Beacon,
    /// Typed address or scanned QR code.
    Manual,
}

/// A report that a device was seen at an address. Says nothing about trust.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PeerSighting {
    /// The device that was seen.
    pub device: DeviceId,
    /// Where it was seen.
    pub addr: SocketAddr,
    /// How it was found.
    pub via: DiscoveryMethod,
}

/// The interface the node uses to find peers.
pub trait Discovery {
    /// Starts finding peers (and announcing, when this device is host).
    fn start(&mut self) -> Result<(), Error>;
    /// Stops all discovery activity.
    fn stop(&mut self) -> Result<(), Error>;
    /// Returns sightings gathered since the last call.
    fn sightings(&mut self) -> Result<Vec<PeerSighting>, Error>;
}

/// Placeholder implementation: every call reports `NotImplemented`.
#[derive(Debug, Default)]
pub struct UnbuiltDiscovery;

impl Discovery for UnbuiltDiscovery {
    fn start(&mut self) -> Result<(), Error> {
        Err(Error::NotImplemented {
            what: "discovery start",
        })
    }
    fn stop(&mut self) -> Result<(), Error> {
        Err(Error::NotImplemented {
            what: "discovery stop",
        })
    }
    fn sightings(&mut self) -> Result<Vec<PeerSighting>, Error> {
        Err(Error::NotImplemented {
            what: "discovery sightings",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_error_has_matching_code_and_display_prefix() {
        let all = [
            (Error::NotImplemented { what: "x" }, "RLY-DS-001"),
            (Error::AlreadyStarted, "RLY-DS-002"),
            (Error::NotStarted, "RLY-DS-003"),
            (Error::BindFailed { reason: "r".into() }, "RLY-DS-004"),
        ];
        for (e, code) in all {
            assert_eq!(e.code().to_string(), code);
            assert!(e.to_string().starts_with(code), "{e}");
        }
    }

    #[test]
    fn unbuilt_discovery_is_not_implemented() {
        let mut d = UnbuiltDiscovery;
        assert!(matches!(d.start(), Err(Error::NotImplemented { .. })));
        assert!(matches!(d.stop(), Err(Error::NotImplemented { .. })));
        assert!(matches!(d.sightings(), Err(Error::NotImplemented { .. })));
    }
}
