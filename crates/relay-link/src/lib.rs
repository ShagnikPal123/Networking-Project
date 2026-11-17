//! Relay `link` module (error prefix `RLY-LK`). Interfaces, paths to a peer, probing, picking the best path.
//!
//! Boundaries and rules: see `docs/design/01-architecture.md` and
//! `docs/agent/modules/link.md`. The state machine and scoring here are pure: no clock, no I/O.

use relay_common::{ErrorCode, Module, RelayError};
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

/// This crate's module tag, used in error codes and health reports.
pub const MODULE: Module = Module::Link;

/// Heartbeat period on the active path.
pub const ACTIVE_HEARTBEAT: Duration = Duration::from_millis(250);
/// Heartbeat period on standby paths.
pub const STANDBY_HEARTBEAT: Duration = Duration::from_secs(2);
/// Consecutive missed heartbeats before an active path becomes suspect.
pub const MISSED_BEFORE_SUSPECT: u32 = 3;
/// How long a suspect path may stay silent before it is declared down.
pub const DOWN_TIMEOUT: Duration = Duration::from_secs(2);

/// Errors raised by the `link` module.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The feature exists in the design but is not built yet.
    #[error("{code} not implemented yet: {what}", code = ErrorCode::new(MODULE, 1))]
    NotImplemented {
        /// The missing feature.
        what: &'static str,
    },
    /// A path ID was not known to the link module.
    #[error("{code} unknown path {path:?}", code = ErrorCode::new(MODULE, 2), path = .0)]
    UnknownPath(PathId),
    /// No path to the peer is reachable.
    #[error("{code} no usable path to peer", code = ErrorCode::new(MODULE, 3))]
    NoUsablePath,
}

impl RelayError for Error {
    fn code(&self) -> ErrorCode {
        match self {
            Error::NotImplemented { .. } => ErrorCode::new(MODULE, 1),
            Error::UnknownPath(_) => ErrorCode::new(MODULE, 2),
            Error::NoUsablePath => ErrorCode::new(MODULE, 3),
        }
    }
}

/// Identifies one path within the link module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PathId(pub u32);

/// What kind of network interface a path runs over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinkKind {
    /// Wired Ethernet.
    Ethernet,
    /// Wi-Fi on a shared router.
    Wifi,
    /// A hotspot run by the host.
    Hotspot,
    /// Anything else.
    Other,
}

impl LinkKind {
    /// Tie-break rank, lower is better: wired, then other, then wireless.
    const fn rank(self) -> u8 {
        match self {
            LinkKind::Ethernet => 0,
            LinkKind::Other => 1,
            LinkKind::Wifi | LinkKind::Hotspot => 2,
        }
    }
}

/// One (local interface address, peer address) pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Path {
    /// Local interface address.
    pub local: IpAddr,
    /// The peer's address on this path.
    pub peer: SocketAddr,
    /// What kind of link this is.
    pub kind: LinkKind,
}

/// Measurements for a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathStats {
    /// Packet loss in parts per thousand.
    pub loss_permille: u16,
    /// Round-trip time in milliseconds.
    pub rtt_ms: u32,
    /// Whether the peer answered recently on this path.
    pub reachable: bool,
}

/// Lifecycle state of a path (protocol section 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PathState {
    /// Newly found, being measured.
    Probing,
    /// The path carrying traffic.
    Active,
    /// Works but is not the best; kept warm.
    Standby,
    /// Active path that missed heartbeats.
    Suspect,
    /// No reply within the timeout.
    Down,
}

/// Something that happened to a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PathEvent {
    /// Probing finished and this path has the best score.
    BecameBest,
    /// Probing finished; the path works but another is better.
    WorksNotBest,
    /// The active path missed [`MISSED_BEFORE_SUSPECT`] heartbeats.
    HeartbeatsMissed,
    /// A heartbeat arrived on a suspect path.
    HeartbeatReturned,
    /// A suspect path stayed silent for [`DOWN_TIMEOUT`].
    TimedOut,
    /// A standby path was promoted because the active one went down.
    Promoted,
    /// The interface of a down path came back.
    InterfaceBack,
}

impl PathState {
    /// Applies `event`. Transitions not in the diagram leave the state unchanged.
    pub fn on_event(self, event: PathEvent) -> PathState {
        use PathEvent as E;
        use PathState as S;
        match (self, event) {
            (S::Probing, E::BecameBest) => S::Active,
            (S::Probing, E::WorksNotBest) => S::Standby,
            (S::Active, E::HeartbeatsMissed) => S::Suspect,
            (S::Suspect, E::HeartbeatReturned) => S::Active,
            (S::Suspect, E::TimedOut) => S::Down,
            (S::Standby, E::Promoted) => S::Active,
            (S::Down, E::InterfaceBack) => S::Probing,
            (state, _) => state,
        }
    }
}

/// Ordering key for a path. Lower is better: reachable, then loss, then delay, then wired.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Score {
    /// 0 when reachable, 1 when not.
    pub unreachable: u8,
    /// Loss in parts per thousand.
    pub loss_permille: u16,
    /// Round-trip time in milliseconds.
    pub rtt_ms: u32,
    /// Link kind rank (wired lowest).
    pub kind_rank: u8,
}

/// Computes the [`Score`] of a path. A smaller score is better.
pub fn score(path: &Path, stats: &PathStats) -> Score {
    Score {
        unreachable: u8::from(!stats.reachable),
        loss_permille: stats.loss_permille,
        rtt_ms: stats.rtt_ms,
        kind_rank: path.kind.rank(),
    }
}

/// Picks the best reachable path, or `None` if none is reachable. Ties keep the earlier entry.
pub fn pick_best(paths: &[(PathId, Path, PathStats)]) -> Option<PathId> {
    paths
        .iter()
        .filter(|(_, _, s)| s.reachable)
        .min_by_key(|(_, p, s)| score(p, s))
        .map(|(id, _, _)| *id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use PathEvent as E;
    use PathState as S;

    fn path(kind: LinkKind) -> Path {
        Path {
            local: "192.168.1.2".parse().unwrap(),
            peer: "192.168.1.3:4000".parse().unwrap(),
            kind,
        }
    }

    fn stats(loss: u16, rtt: u32, reachable: bool) -> PathStats {
        PathStats {
            loss_permille: loss,
            rtt_ms: rtt,
            reachable,
        }
    }

    #[test]
    fn every_error_has_matching_code_and_display_prefix() {
        let all = [
            (Error::NotImplemented { what: "x" }, "RLY-LK-001"),
            (Error::UnknownPath(PathId(1)), "RLY-LK-002"),
            (Error::NoUsablePath, "RLY-LK-003"),
        ];
        for (e, code) in all {
            assert_eq!(e.code().to_string(), code);
            assert!(e.to_string().starts_with(code), "{e}");
        }
    }

    #[test]
    fn diagram_transitions() {
        assert_eq!(S::Probing.on_event(E::BecameBest), S::Active);
        assert_eq!(S::Probing.on_event(E::WorksNotBest), S::Standby);
        assert_eq!(S::Active.on_event(E::HeartbeatsMissed), S::Suspect);
        assert_eq!(S::Suspect.on_event(E::HeartbeatReturned), S::Active);
        assert_eq!(S::Suspect.on_event(E::TimedOut), S::Down);
        assert_eq!(S::Standby.on_event(E::Promoted), S::Active);
        assert_eq!(S::Down.on_event(E::InterfaceBack), S::Probing);
    }

    #[test]
    fn invalid_transitions_leave_state_unchanged() {
        let states = [S::Probing, S::Active, S::Standby, S::Suspect, S::Down];
        let events = [
            E::BecameBest,
            E::WorksNotBest,
            E::HeartbeatsMissed,
            E::HeartbeatReturned,
            E::TimedOut,
            E::Promoted,
            E::InterfaceBack,
        ];
        let valid = [
            (S::Probing, E::BecameBest),
            (S::Probing, E::WorksNotBest),
            (S::Active, E::HeartbeatsMissed),
            (S::Suspect, E::HeartbeatReturned),
            (S::Suspect, E::TimedOut),
            (S::Standby, E::Promoted),
            (S::Down, E::InterfaceBack),
        ];
        for s in states {
            for e in events {
                if !valid.contains(&(s, e)) {
                    assert_eq!(s.on_event(e), s, "{s:?} + {e:?}");
                }
            }
        }
    }

    #[test]
    fn failover_walk() {
        let s = S::Probing
            .on_event(E::BecameBest)
            .on_event(E::HeartbeatsMissed)
            .on_event(E::TimedOut)
            .on_event(E::InterfaceBack);
        assert_eq!(s, S::Probing);
    }

    #[test]
    fn timing_constants_match_design() {
        assert_eq!(ACTIVE_HEARTBEAT, Duration::from_millis(250));
        assert_eq!(STANDBY_HEARTBEAT, Duration::from_secs(2));
        assert_eq!(MISSED_BEFORE_SUSPECT, 3);
        assert_eq!(DOWN_TIMEOUT, Duration::from_secs(2));
    }

    #[test]
    fn pick_best_empty_and_all_unreachable() {
        assert_eq!(pick_best(&[]), None);
        let p = [(PathId(1), path(LinkKind::Ethernet), stats(0, 1, false))];
        assert_eq!(pick_best(&p), None);
    }

    #[test]
    fn reachable_beats_everything() {
        let p = [
            (PathId(1), path(LinkKind::Ethernet), stats(0, 1, false)),
            (PathId(2), path(LinkKind::Wifi), stats(500, 900, true)),
        ];
        assert_eq!(pick_best(&p), Some(PathId(2)));
    }

    #[test]
    fn loss_then_rtt_then_wired() {
        let lossy = [
            (PathId(1), path(LinkKind::Ethernet), stats(10, 1, true)),
            (PathId(2), path(LinkKind::Wifi), stats(0, 50, true)),
        ];
        assert_eq!(pick_best(&lossy), Some(PathId(2)));
        let slow = [
            (PathId(1), path(LinkKind::Ethernet), stats(0, 20, true)),
            (PathId(2), path(LinkKind::Wifi), stats(0, 5, true)),
        ];
        assert_eq!(pick_best(&slow), Some(PathId(2)));
        let tie = [
            (PathId(1), path(LinkKind::Wifi), stats(0, 5, true)),
            (PathId(2), path(LinkKind::Ethernet), stats(0, 5, true)),
            (PathId(3), path(LinkKind::Hotspot), stats(0, 5, true)),
        ];
        assert_eq!(pick_best(&tie), Some(PathId(2)));
    }
}
