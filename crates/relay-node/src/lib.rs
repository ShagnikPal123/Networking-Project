//! Relay `node` module (error prefix `RLY-ND`). Starts modules, routes events, reports health. No protocol logic.
//!
//! Boundaries and rules: see `docs/design/01-architecture.md` and
//! `docs/agent/modules/node.md`.

use relay_common::{DeviceId, Epoch, ErrorCode, Health, HealthReport, Module, RelayError, SiteId};

/// This crate's module tag, used in error codes and health reports.
pub const MODULE: Module = Module::Node;

/// Errors raised by the `node` module.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The feature exists in the design but is not built yet.
    #[error("{code} not implemented yet: {what}", code = ErrorCode::new(MODULE, 1))]
    NotImplemented {
        /// The missing feature.
        what: &'static str,
    },
    /// The node has not been started.
    #[error("{code} node is not running", code = ErrorCode::new(MODULE, 2))]
    NotRunning,
}

impl RelayError for Error {
    fn code(&self) -> ErrorCode {
        match self {
            Error::NotImplemented { .. } => ErrorCode::new(MODULE, 1),
            Error::NotRunning => ErrorCode::new(MODULE, 2),
        }
    }
}

/// News that modules send up to the node, which routes it to the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Discovery saw a device.
    PeerFound {
        /// The device that was seen.
        device: DeviceId,
    },
    /// A session to a peer is established.
    PeerConnected {
        /// The peer.
        device: DeviceId,
    },
    /// A session to a peer ended.
    PeerLost {
        /// The peer.
        device: DeviceId,
    },
    /// A path to a peer went down.
    PathDown {
        /// The peer the path led to.
        device: DeviceId,
    },
    /// A module changed its health.
    HealthChanged(HealthReport),
    /// The site host changed.
    HostChanged {
        /// The site concerned.
        site: SiteId,
        /// The new host.
        host: DeviceId,
        /// The epoch of the new host.
        epoch: Epoch,
    },
}

/// One health entry per module, plus the latest non-ok code.
#[derive(Debug, Clone)]
pub struct HealthBoard {
    reports: Vec<HealthReport>,
    last_code: Option<ErrorCode>,
}

impl Default for HealthBoard {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthBoard {
    /// Creates a board with every module `Ok`.
    pub fn new() -> Self {
        Self {
            reports: Module::ALL
                .iter()
                .map(|&module| HealthReport {
                    module,
                    health: Health::Ok,
                })
                .collect(),
            last_code: None,
        }
    }

    /// Records `health` for `module`. A non-ok health becomes the latest code shown.
    pub fn set(&mut self, module: Module, health: Health) {
        match &health {
            Health::Ok => {}
            Health::Degraded(c) | Health::Down(c) => self.last_code = Some(*c),
        }
        if let Some(r) = self.reports.iter_mut().find(|r| r.module == module) {
            r.health = health;
        }
    }

    /// Returns the current health of `module`.
    pub fn get(&self, module: Module) -> &Health {
        &self
            .reports
            .iter()
            .find(|r| r.module == module)
            .expect("board holds every module")
            .health
    }

    /// Returns all reports in [`Module::ALL`] order.
    pub fn snapshot(&self) -> Vec<HealthReport> {
        self.reports.clone()
    }

    /// Renders `ID ●  PR ●  LK ◐  SS ○`, then a second line with the latest non-ok code, if any.
    pub fn render_strip(&self) -> String {
        let strip: Vec<String> = self
            .reports
            .iter()
            .map(|r| {
                let light = match r.health {
                    Health::Ok => '●',
                    Health::Degraded(_) => '◐',
                    Health::Down(_) => '○',
                };
                format!("{} {}", r.module.prefix(), light)
            })
            .collect();
        let mut out = strip.join("  ");
        if let Some(code) = self.last_code {
            out.push_str(&format!("\n└─ {code}"));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_error_has_matching_code_and_display_prefix() {
        let all = [
            (Error::NotImplemented { what: "x" }, "RLY-ND-001"),
            (Error::NotRunning, "RLY-ND-002"),
        ];
        for (e, code) in all {
            assert_eq!(e.code().to_string(), code);
            assert!(e.to_string().starts_with(code), "{e}");
        }
    }

    #[test]
    fn new_board_is_all_ok() {
        let b = HealthBoard::new();
        assert_eq!(b.snapshot().len(), Module::ALL.len());
        assert!(Module::ALL.iter().all(|&m| *b.get(m) == Health::Ok));
        let strip = b.render_strip();
        assert!(!strip.contains('\n'));
        assert!(strip.starts_with("ID ●  PR ●  ST ●"));
    }

    #[test]
    fn set_and_get_and_strip() {
        let mut b = HealthBoard::new();
        let lk = ErrorCode::new(Module::Link, 3);
        b.set(Module::Link, Health::Degraded(lk));
        b.set(
            Module::Session,
            Health::Down(ErrorCode::new(Module::Session, 2)),
        );
        assert_eq!(*b.get(Module::Link), Health::Degraded(lk));
        let strip = b.render_strip();
        assert!(strip.contains("LK ◐  SS ○"), "{strip}");
        assert!(strip.ends_with("\n└─ RLY-SS-002"), "{strip}");
    }

    #[test]
    fn recovery_keeps_latest_code_line_only_when_set() {
        let mut b = HealthBoard::new();
        b.set(Module::Link, Health::Ok);
        assert!(!b.render_strip().contains("RLY"));
    }
}
