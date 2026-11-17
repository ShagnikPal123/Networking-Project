//! Relay `sim` harness: several nodes in one process with a scriptable network.
//!
//! Everything is deterministic. Time is virtual ([`SimClock`]) and only moves when
//! [`SimNet::advance`] is called. There are no threads, sleeps, or system randomness.
//! This is a test tool, so its errors ([`SimError`]) carry no `RLY-` code.

use std::collections::{BTreeMap, BTreeSet};

use relay_common::DeviceId;

/// Errors from the simulated network.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SimError {
    /// The node was never added.
    #[error("unknown node")]
    UnknownNode,
    /// The link was never added, or does not join the given nodes.
    #[error("unknown link {0:?}")]
    UnknownLink(LinkId),
    /// The link is down, so nothing can be sent on it.
    #[error("link {0:?} is down")]
    LinkDown(LinkId),
    /// The sending node has been killed.
    #[error("node is dead")]
    NodeDead,
}

/// Virtual time in milliseconds. Only advances when told.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SimClock {
    now_ms: u64,
}

impl SimClock {
    /// A clock at time zero.
    pub fn new() -> Self {
        Self::default()
    }

    /// The current virtual time in milliseconds.
    pub fn now_ms(&self) -> u64 {
        self.now_ms
    }

    /// Move time forward by `ms`.
    pub fn advance(&mut self, ms: u64) {
        self.now_ms += ms;
    }
}

/// Identifies one link. Two links may join the same pair of nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LinkId(pub u32);

/// What a simulated link stands for (a label only; no behavior depends on it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SimLinkKind {
    /// Wired Ethernet.
    Ethernet,
    /// Wi-Fi.
    Wifi,
    /// A phone or host hotspot.
    Hotspot,
}

/// A packet handed to a node by [`SimNet::recv`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delivered {
    /// The sender.
    pub from: DeviceId,
    /// The link it travelled on.
    pub via: LinkId,
    /// Virtual time of delivery in milliseconds.
    pub at_ms: u64,
    /// The payload.
    pub bytes: Vec<u8>,
}

struct Link {
    a: DeviceId,
    b: DeviceId,
    kind: SimLinkKind,
    delay_ms: u64,
    up: bool,
    /// Bumped each time the link goes down, so in-flight packets can be told apart.
    generation: u64,
    drop_every: u64,
    sent: u64,
}

struct Node {
    alive: bool,
    generation: u64,
    inbox: Vec<Delivered>,
}

struct InFlight {
    due_ms: u64,
    seq: u64,
    from: DeviceId,
    to: DeviceId,
    via: LinkId,
    link_gen: u64,
    to_gen: u64,
    bytes: Vec<u8>,
}

/// The simulated network: nodes, links, and packets in flight.
pub struct SimNet {
    clock: SimClock,
    nodes: BTreeMap<DeviceId, Node>,
    links: BTreeMap<LinkId, Link>,
    flight: Vec<InFlight>,
    next_link: u32,
    next_seq: u64,
}

impl Default for SimNet {
    fn default() -> Self {
        Self::new()
    }
}

impl SimNet {
    /// An empty network at time zero.
    pub fn new() -> Self {
        Self {
            clock: SimClock::new(),
            nodes: BTreeMap::new(),
            links: BTreeMap::new(),
            flight: Vec::new(),
            next_link: 0,
            next_seq: 0,
        }
    }

    /// The current virtual time in milliseconds.
    pub fn now_ms(&self) -> u64 {
        self.clock.now_ms()
    }

    /// Add a live node. Adding the same node twice does nothing.
    pub fn add_node(&mut self, id: DeviceId) {
        self.nodes.entry(id).or_insert(Node {
            alive: true,
            generation: 0,
            inbox: Vec::new(),
        });
    }

    /// Join `a` and `b` with a new link that starts up. Both nodes must exist.
    pub fn add_link(
        &mut self,
        a: DeviceId,
        b: DeviceId,
        kind: SimLinkKind,
        delay_ms: u64,
    ) -> Result<LinkId, SimError> {
        if !self.nodes.contains_key(&a) || !self.nodes.contains_key(&b) {
            return Err(SimError::UnknownNode);
        }
        let id = LinkId(self.next_link);
        self.next_link += 1;
        self.links.insert(
            id,
            Link {
                a,
                b,
                kind,
                delay_ms,
                up: true,
                generation: 0,
                drop_every: 0,
                sent: 0,
            },
        );
        Ok(id)
    }

    /// The label given to a link when it was added.
    pub fn link_kind(&self, link: LinkId) -> Result<SimLinkKind, SimError> {
        self.links
            .get(&link)
            .map(|l| l.kind)
            .ok_or(SimError::UnknownLink(link))
    }

    /// Bring a link up or down. Taking it down loses every packet still in flight on it.
    pub fn set_link_up(&mut self, link: LinkId, up: bool) -> Result<(), SimError> {
        let l = self
            .links
            .get_mut(&link)
            .ok_or(SimError::UnknownLink(link))?;
        if l.up && !up {
            l.generation += 1;
        }
        l.up = up;
        Ok(())
    }

    /// Change a link's one-way delay. Applies to packets sent from now on.
    pub fn set_delay(&mut self, link: LinkId, delay_ms: u64) -> Result<(), SimError> {
        self.links
            .get_mut(&link)
            .ok_or(SimError::UnknownLink(link))?
            .delay_ms = delay_ms;
        Ok(())
    }

    /// Drop every `n`th packet sent on a link (`0` turns dropping off). Deterministic.
    pub fn set_drop_every(&mut self, link: LinkId, n: u64) -> Result<(), SimError> {
        let l = self
            .links
            .get_mut(&link)
            .ok_or(SimError::UnknownLink(link))?;
        l.drop_every = n;
        l.sent = 0;
        Ok(())
    }

    /// Kill a node: it receives nothing, cannot send, and loses packets in flight to it.
    pub fn kill_node(&mut self, id: DeviceId) -> Result<(), SimError> {
        let n = self.nodes.get_mut(&id).ok_or(SimError::UnknownNode)?;
        if n.alive {
            n.alive = false;
            n.generation += 1;
        }
        Ok(())
    }

    /// Bring a killed node back. Its inbox starts empty.
    pub fn revive_node(&mut self, id: DeviceId) -> Result<(), SimError> {
        let n = self.nodes.get_mut(&id).ok_or(SimError::UnknownNode)?;
        n.alive = true;
        n.inbox.clear();
        Ok(())
    }

    /// Whether a node is currently alive.
    pub fn is_alive(&self, id: DeviceId) -> Result<bool, SimError> {
        self.nodes
            .get(&id)
            .map(|n| n.alive)
            .ok_or(SimError::UnknownNode)
    }

    /// Send `bytes` from `from` to `to` over `via`. Fails at once if the link is down or the
    /// sender is dead. A packet removed by the drop pattern returns `Ok` but never arrives.
    pub fn send(
        &mut self,
        from: DeviceId,
        to: DeviceId,
        via: LinkId,
        bytes: Vec<u8>,
    ) -> Result<(), SimError> {
        let sender = self.nodes.get(&from).ok_or(SimError::UnknownNode)?;
        let receiver = self.nodes.get(&to).ok_or(SimError::UnknownNode)?;
        if !sender.alive {
            return Err(SimError::NodeDead);
        }
        let to_gen = receiver.generation;
        let l = self.links.get_mut(&via).ok_or(SimError::UnknownLink(via))?;
        let joins = (l.a == from && l.b == to) || (l.b == from && l.a == to);
        if !joins {
            return Err(SimError::UnknownLink(via));
        }
        if !l.up {
            return Err(SimError::LinkDown(via));
        }
        l.sent += 1;
        if l.drop_every > 0 && l.sent % l.drop_every == 0 {
            return Ok(());
        }
        let seq = self.next_seq;
        self.next_seq += 1;
        self.flight.push(InFlight {
            due_ms: self.clock.now_ms() + l.delay_ms,
            seq,
            from,
            to,
            via,
            link_gen: l.generation,
            to_gen,
            bytes,
        });
        Ok(())
    }

    /// Move time forward by `ms`, delivering due packets in order of (due time, send order).
    /// A packet is lost if its link went down, or its target was killed, after it was sent.
    pub fn advance(&mut self, ms: u64) {
        let target = self.clock.now_ms() + ms;
        let (mut due, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut self.flight)
            .into_iter()
            .partition(|p| p.due_ms <= target);
        self.flight = rest;
        due.sort_by_key(|p| (p.due_ms, p.seq));
        for p in due {
            let link_ok = self
                .links
                .get(&p.via)
                .is_some_and(|l| l.up && l.generation == p.link_gen);
            if let Some(n) = self.nodes.get_mut(&p.to) {
                if link_ok && n.alive && n.generation == p.to_gen {
                    n.inbox.push(Delivered {
                        from: p.from,
                        via: p.via,
                        at_ms: p.due_ms,
                        bytes: p.bytes,
                    });
                }
            }
        }
        self.clock.advance(ms);
    }

    /// Take everything delivered to `node` so far, oldest first.
    pub fn recv(&mut self, node: DeviceId) -> Vec<Delivered> {
        self.nodes
            .get_mut(&node)
            .map(|n| std::mem::take(&mut n.inbox))
            .unwrap_or_default()
    }

    /// Number of packets currently in flight.
    pub fn in_flight(&self) -> usize {
        self.flight.len()
    }

    /// The ids of all nodes, in sorted order.
    pub fn nodes(&self) -> BTreeSet<DeviceId> {
        self.nodes.keys().copied().collect()
    }
}

/// One thing a [`Scenario`] can do to the network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Set a link up or down.
    LinkUp(LinkId, bool),
    /// Change a link's delay in milliseconds.
    Delay(LinkId, u64),
    /// Drop every Nth packet on a link (0 = off).
    DropEvery(LinkId, u64),
    /// Kill a node.
    Kill(DeviceId),
    /// Revive a node.
    Revive(DeviceId),
}

/// A script of [`Action`]s at virtual times, applied while time advances.
#[derive(Debug, Clone, Default)]
pub struct Scenario {
    steps: Vec<(u64, Action)>,
    next: usize,
    sorted: bool,
}

impl Scenario {
    /// An empty scenario.
    pub fn new() -> Self {
        Self::default()
    }

    /// Schedule `action` at absolute virtual time `at_ms`. Same-time actions run in the order added.
    pub fn at(mut self, at_ms: u64, action: Action) -> Self {
        self.steps.push((at_ms, action));
        self.sorted = false;
        self
    }

    /// Advance `net` by `ms`, applying each scheduled action exactly at its time.
    /// Actions due at the new end time are applied before returning.
    pub fn advance(&mut self, net: &mut SimNet, ms: u64) -> Result<(), SimError> {
        if !self.sorted {
            self.steps.sort_by_key(|(t, _)| *t); // stable
            self.sorted = true;
        }
        let end = net.now_ms() + ms;
        while self.next < self.steps.len() && self.steps[self.next].0 <= end {
            let (t, action) = self.steps[self.next].clone();
            self.next += 1;
            net.advance(t.saturating_sub(net.now_ms()));
            match action {
                Action::LinkUp(l, up) => net.set_link_up(l, up)?,
                Action::Delay(l, d) => net.set_delay(l, d)?,
                Action::DropEvery(l, n) => net.set_drop_every(l, n)?,
                Action::Kill(n) => net.kill_node(n)?,
                Action::Revive(n) => net.revive_node(n)?,
            }
        }
        net.advance(end.saturating_sub(net.now_ms()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> DeviceId {
        DeviceId([n; 32])
    }

    fn pair() -> (SimNet, DeviceId, DeviceId) {
        let mut net = SimNet::new();
        net.add_node(id(1));
        net.add_node(id(2));
        (net, id(1), id(2))
    }

    #[test]
    fn delivers_after_delay() {
        let (mut net, a, b) = pair();
        let l = net.add_link(a, b, SimLinkKind::Wifi, 50).unwrap();
        net.send(a, b, l, b"hi".to_vec()).unwrap();
        net.advance(49);
        assert!(net.recv(b).is_empty());
        net.advance(1);
        let got = net.recv(b);
        assert_eq!(got.len(), 1);
        assert_eq!((got[0].from, got[0].via, got[0].at_ms), (a, l, 50));
        assert_eq!(got[0].bytes, b"hi");
    }

    #[test]
    fn down_link_refuses_send() {
        let (mut net, a, b) = pair();
        let l = net.add_link(a, b, SimLinkKind::Ethernet, 1).unwrap();
        net.set_link_up(l, false).unwrap();
        assert_eq!(net.send(a, b, l, vec![1]), Err(SimError::LinkDown(l)));
        net.advance(10);
        assert!(net.recv(b).is_empty());
    }

    #[test]
    fn in_flight_lost_when_link_drops_even_if_it_comes_back() {
        let (mut net, a, b) = pair();
        let l = net.add_link(a, b, SimLinkKind::Wifi, 100).unwrap();
        net.send(a, b, l, vec![1]).unwrap();
        net.advance(10);
        net.set_link_up(l, false).unwrap();
        net.set_link_up(l, true).unwrap();
        net.advance(200);
        assert!(net.recv(b).is_empty());
    }

    #[test]
    fn killed_node_receives_nothing() {
        let (mut net, a, b) = pair();
        let l = net.add_link(a, b, SimLinkKind::Wifi, 10).unwrap();
        net.send(a, b, l, vec![1]).unwrap();
        net.kill_node(b).unwrap();
        net.advance(20);
        assert!(net.recv(b).is_empty());
        net.send(a, b, l, vec![2]).unwrap();
        net.advance(20);
        assert!(net.recv(b).is_empty());
        assert_eq!(net.send(b, a, l, vec![3]), Err(SimError::NodeDead));
    }

    #[test]
    fn revived_node_receives_again() {
        let (mut net, a, b) = pair();
        let l = net.add_link(a, b, SimLinkKind::Wifi, 10).unwrap();
        net.kill_node(b).unwrap();
        net.revive_node(b).unwrap();
        net.send(a, b, l, vec![1]).unwrap();
        net.advance(10);
        assert_eq!(net.recv(b).len(), 1);
    }

    #[test]
    fn two_links_same_pair_are_independent() {
        let (mut net, a, b) = pair();
        let wifi = net.add_link(a, b, SimLinkKind::Wifi, 30).unwrap();
        let eth = net.add_link(a, b, SimLinkKind::Ethernet, 5).unwrap();
        net.send(a, b, wifi, vec![1]).unwrap();
        net.set_link_up(wifi, false).unwrap();
        net.send(a, b, eth, vec![2]).unwrap();
        net.advance(40);
        let got = net.recv(b);
        assert_eq!(got.len(), 1);
        assert_eq!((got[0].via, got[0].bytes.clone()), (eth, vec![2]));
    }

    #[test]
    fn drop_pattern_is_deterministic() {
        let (mut net, a, b) = pair();
        let l = net.add_link(a, b, SimLinkKind::Hotspot, 1).unwrap();
        net.set_drop_every(l, 3).unwrap();
        for i in 1..=6u8 {
            net.send(a, b, l, vec![i]).unwrap();
        }
        net.advance(1);
        let got: Vec<u8> = net.recv(b).iter().map(|d| d.bytes[0]).collect();
        assert_eq!(got, vec![1, 2, 4, 5]);
    }

    #[test]
    fn wrong_endpoints_rejected() {
        let (mut net, a, b) = pair();
        net.add_node(id(3));
        let l = net.add_link(a, b, SimLinkKind::Wifi, 1).unwrap();
        assert_eq!(net.send(a, id(3), l, vec![]), Err(SimError::UnknownLink(l)));
        assert_eq!(net.send(a, id(9), l, vec![]), Err(SimError::UnknownNode));
    }

    fn run_once() -> Vec<(u64, u8, u32)> {
        let (mut net, a, b) = pair();
        let wifi = net.add_link(a, b, SimLinkKind::Wifi, 20).unwrap();
        let eth = net.add_link(a, b, SimLinkKind::Ethernet, 5).unwrap();
        let mut sc = Scenario::new()
            .at(100, Action::LinkUp(wifi, false))
            .at(150, Action::Delay(eth, 40));
        let mut out = Vec::new();
        for i in 0..20u8 {
            let via = if i < 10 { wifi } else { eth };
            let _ = net.send(a, b, via, vec![i]);
            sc.advance(&mut net, 10).unwrap();
            for d in net.recv(b) {
                out.push((d.at_ms, d.bytes[0], d.via.0));
            }
        }
        out
    }

    #[test]
    fn scenario_fires_at_virtual_time() {
        let (mut net, a, b) = pair();
        let l = net.add_link(a, b, SimLinkKind::Wifi, 10).unwrap();
        let mut sc = Scenario::new().at(500, Action::LinkUp(l, false));
        sc.advance(&mut net, 499).unwrap();
        assert!(net.send(a, b, l, vec![1]).is_ok());
        sc.advance(&mut net, 1).unwrap();
        assert_eq!(net.now_ms(), 500);
        assert_eq!(net.send(a, b, l, vec![2]), Err(SimError::LinkDown(l)));
    }

    #[test]
    fn scenario_mid_step_action_loses_in_flight() {
        let (mut net, a, b) = pair();
        let l = net.add_link(a, b, SimLinkKind::Wifi, 100).unwrap();
        net.send(a, b, l, vec![1]).unwrap();
        let mut sc = Scenario::new().at(50, Action::LinkUp(l, false));
        sc.advance(&mut net, 300).unwrap();
        assert!(net.recv(b).is_empty());
    }

    #[test]
    fn same_scenario_twice_is_identical() {
        let first = run_once();
        assert!(!first.is_empty());
        assert_eq!(first, run_once());
    }
}
