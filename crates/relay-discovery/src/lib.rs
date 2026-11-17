//! Relay `discovery` module (error prefix `RLY-DS`). Finding sites and peers on the LAN. Never decides trust.
//!
//! Boundaries and rules: see `docs/design/01-architecture.md` and
//! `docs/agent/modules/discovery.md`. Discovery only says "something is here"; it never grants
//! trust (protocol section 2).
//!
//! Parts, so a failure can be pinned to one of them:
//!
//! - this file: the public vocabulary ([`Beacon`], [`PeerSighting`], [`Discovery`], [`Error`]);
//! - [`wire`]: the beacon's byte layout and the signing hooks. Pure, no I/O;
//! - [`udp`]: the real UDP sender and listener ([`UdpBeaconDiscovery`]).
//!
//! mDNS (the second method in the design) is not built yet.

use relay_common::{DeviceId, Epoch, ErrorCode, Module, RelayError, SiteId};
use std::net::SocketAddr;

pub mod udp;
pub mod wire;

pub use udp::{BeaconConfig, UdpBeaconDiscovery, DEFAULT_BEACON_PORT};
pub use wire::{BeaconSigner, BeaconVerifier, BEACON_LEN, BEACON_VERSION};

#[cfg(test)]
mod testkit;

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
    /// A datagram is not a beacon: wrong size, wrong magic, or a zero port.
    #[error("{code} malformed beacon: {reason}", code = ErrorCode::new(MODULE, 5))]
    MalformedBeacon {
        /// What was wrong.
        reason: &'static str,
    },
    /// A beacon was written by a wire version this build does not read.
    #[error("{code} unsupported beacon version {version}", code = ErrorCode::new(MODULE, 6))]
    UnsupportedVersion {
        /// The version found in the datagram.
        version: u16,
    },
    /// A beacon's signature or key does not match the device it names.
    #[error("{code} beacon signature is not valid", code = ErrorCode::new(MODULE, 7))]
    BadSignature,
    /// The beacon could not be sent to at least one target.
    #[error("{code} could not send beacon: {reason}", code = ErrorCode::new(MODULE, 8))]
    SendFailed {
        /// Human-readable cause.
        reason: String,
    },
    /// The signer refused to sign this device's beacon.
    #[error("{code} could not sign beacon: {reason}", code = ErrorCode::new(MODULE, 9))]
    SignFailed {
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
            Error::MalformedBeacon { .. } => ErrorCode::new(MODULE, 5),
            Error::UnsupportedVersion { .. } => ErrorCode::new(MODULE, 6),
            Error::BadSignature => ErrorCode::new(MODULE, 7),
            Error::SendFailed { .. } => ErrorCode::new(MODULE, 8),
            Error::SignFailed { .. } => ErrorCode::new(MODULE, 9),
        }
    }
}

/// The content of a UDP broadcast beacon: `{site ID, host device ID, epoch, port}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Beacon {
    /// The site being announced.
    pub site: SiteId,
    /// The device currently acting as host.
    pub host: DeviceId,
    /// The host epoch; higher wins.
    pub epoch: Epoch,
    /// UDP port the host accepts sessions on.
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
            (Error::MalformedBeacon { reason: "r" }, "RLY-DS-005"),
            (Error::UnsupportedVersion { version: 9 }, "RLY-DS-006"),
            (Error::BadSignature, "RLY-DS-007"),
            (Error::SendFailed { reason: "r".into() }, "RLY-DS-008"),
            (Error::SignFailed { reason: "r".into() }, "RLY-DS-009"),
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
