//! Shared vocabulary for every Relay module: module tags, error codes, health states.
//!
//! This crate holds no logic and depends on nothing in the workspace. Every failure in Relay
//! carries an [`ErrorCode`] like `RLY-LK-003`, so it can be traced to exactly one module.

use std::fmt;

/// One tag per module in `docs/design/01-architecture.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Module {
    /// Device keys, site keys, certificates.
    Identity,
    /// Wire message types and encoding.
    Proto,
    /// Local persistence.
    Store,
    /// Finding sites and peers.
    Discovery,
    /// Interfaces, paths, failover.
    Link,
    /// Encrypted, authenticated peer sessions.
    Session,
    /// Joins, roster, host handoff.
    Membership,
    /// File transfer.
    Transfer,
    /// Folder grants and browsing.
    Share,
    /// Hash-chained audit log.
    Audit,
    /// Wiring, events, health.
    Node,
}

impl Module {
    /// Every module, in dependency order (base first).
    pub const ALL: [Module; 11] = [
        Module::Identity,
        Module::Proto,
        Module::Store,
        Module::Discovery,
        Module::Link,
        Module::Session,
        Module::Membership,
        Module::Transfer,
        Module::Share,
        Module::Audit,
        Module::Node,
    ];

    /// The two-letter prefix used in error codes.
    pub const fn prefix(self) -> &'static str {
        match self {
            Module::Identity => "ID",
            Module::Proto => "PR",
            Module::Store => "ST",
            Module::Discovery => "DS",
            Module::Link => "LK",
            Module::Session => "SS",
            Module::Membership => "MB",
            Module::Transfer => "TX",
            Module::Share => "SH",
            Module::Audit => "AU",
            Module::Node => "ND",
        }
    }
}

/// A stable error identifier, shown as `RLY-<MODULE>-<NUMBER>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ErrorCode {
    /// The module that raised the error.
    pub module: Module,
    /// The error number within that module. Numbers are never reused.
    pub number: u16,
}

impl ErrorCode {
    /// Builds a code for `module` with the given number.
    pub const fn new(module: Module, number: u16) -> Self {
        Self { module, number }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RLY-{}-{:03}", self.module.prefix(), self.number)
    }
}

/// Implemented by every module's error type so the original code survives wrapping.
pub trait RelayError: std::error::Error {
    /// The code of the module that first raised this error.
    fn code(&self) -> ErrorCode;
}

/// A device's identity: a hash of its public key. Built by `relay-identity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DeviceId(pub [u8; 32]);

/// A site's identity: a hash of the site public key. Built by `relay-identity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SiteId(pub [u8; 32]);

/// Host-change counter. Goes up by one at every host change; the higher epoch always wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Epoch(pub u64);

impl Epoch {
    /// The epoch that follows this one.
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// How a module is doing right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Health {
    /// Working normally.
    Ok,
    /// Working with reduced ability, for example on a backup link.
    Degraded(ErrorCode),
    /// Not working. Modules that do not depend on it must keep running.
    Down(ErrorCode),
}

/// One module's entry in the health strip shown by the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthReport {
    /// Which module is reporting.
    pub module: Module,
    /// Its current state.
    pub health: Health,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn code_formats_with_prefix_and_padding() {
        assert_eq!(ErrorCode::new(Module::Link, 3).to_string(), "RLY-LK-003");
    }

    #[test]
    fn epoch_orders_and_advances() {
        assert!(Epoch(2).next() > Epoch(2));
        assert_eq!(Epoch::default().next(), Epoch(1));
    }

    #[test]
    fn prefixes_are_unique() {
        let set: HashSet<_> = Module::ALL.iter().map(|m| m.prefix()).collect();
        assert_eq!(set.len(), Module::ALL.len());
    }
}
