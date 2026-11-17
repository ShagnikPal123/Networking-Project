//! The real UDP beacon: one socket, one background thread.
//!
//! The thread does two jobs on the same socket:
//!
//! - **send** this device's signed beacon to every target once per interval (only when an
//!   announcement was set with [`UdpBeaconDiscovery::announce`]);
//! - **listen** for other devices' beacons and queue a [`PeerSighting`] for each valid one.
//!
//! Anything that does not decode and verify is dropped silently, so junk or hostile datagrams
//! cannot stop discovery. The queue is bounded, so a flood cannot grow memory.

use crate::wire::{decode_beacon, encode_beacon, BeaconSigner, BeaconVerifier, BEACON_LEN};
use crate::{Beacon, Discovery, DiscoveryMethod, Error, PeerSighting, BEACON_INTERVAL_MS, MODULE};
use relay_common::{DeviceId, ErrorCode, Health};
use std::io::ErrorKind;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// UDP port beacons are sent to and listened on by default.
pub const DEFAULT_BEACON_PORT: u16 = 47_470;

/// Most sightings kept between two [`Discovery::sightings`] calls. Extra ones are dropped.
pub const MAX_PENDING_SIGHTINGS: usize = 1_024;

/// Longest the thread waits for a datagram before checking for stop or a due beacon.
const MAX_POLL: Duration = Duration::from_millis(50);

/// Where to listen, where to send, and how often.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeaconConfig {
    /// Local address the socket binds to. Port 0 picks a free port.
    pub bind: SocketAddr,
    /// Addresses each beacon is sent to. Usually one broadcast address.
    pub targets: Vec<SocketAddr>,
    /// Time between two beacons.
    pub interval: Duration,
}

impl Default for BeaconConfig {
    /// All interfaces, limited broadcast, [`DEFAULT_BEACON_PORT`], every 2 s.
    fn default() -> Self {
        Self {
            bind: SocketAddr::from((Ipv4Addr::UNSPECIFIED, DEFAULT_BEACON_PORT)),
            targets: vec![SocketAddr::from((Ipv4Addr::BROADCAST, DEFAULT_BEACON_PORT))],
            interval: Duration::from_millis(BEACON_INTERVAL_MS),
        }
    }
}

/// State shared between the owner and the background thread.
struct Shared {
    stop: AtomicBool,
    send_failed: AtomicBool,
    pending: Mutex<Vec<PeerSighting>>,
}

impl Shared {
    fn pending(&self) -> MutexGuard<'_, Vec<PeerSighting>> {
        // The queue is only pushed to or drained, so a poisoned lock still holds valid data.
        self.pending.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// What exists only while discovery is running.
struct Running {
    shared: Arc<Shared>,
    thread: JoinHandle<()>,
    local_addr: SocketAddr,
}

/// LAN discovery by signed UDP broadcast beacon (protocol section 2, method 2).
pub struct UdpBeaconDiscovery {
    config: BeaconConfig,
    verifier: Arc<dyn BeaconVerifier>,
    announce: Option<(Beacon, Arc<dyn BeaconSigner>)>,
    own_device: Option<DeviceId>,
    running: Option<Running>,
}

impl std::fmt::Debug for UdpBeaconDiscovery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UdpBeaconDiscovery")
            .field("config", &self.config)
            .field("announcing", &self.announce.as_ref().map(|(b, _)| b))
            .field("running", &self.running.is_some())
            .finish()
    }
}

impl UdpBeaconDiscovery {
    /// A listen-only discovery. Nothing is opened until [`Discovery::start`].
    pub fn new(config: BeaconConfig, verifier: Arc<dyn BeaconVerifier>) -> Self {
        Self {
            config,
            verifier,
            announce: None,
            own_device: None,
            running: None,
        }
    }

    /// Also announce `beacon`, signed by `signer`. Beacons naming `beacon.host` are then
    /// ignored on receive, so a device never sights itself. Takes effect at the next start.
    pub fn announce(mut self, beacon: Beacon, signer: Arc<dyn BeaconSigner>) -> Self {
        self.own_device = Some(beacon.host);
        self.announce = Some((beacon, signer));
        self
    }

    /// Ignore beacons naming `device` (this device), without announcing anything.
    pub fn ignore_device(mut self, device: DeviceId) -> Self {
        self.own_device = Some(device);
        self
    }

    /// The address the socket is bound to, once running. Useful when the config asked for port 0.
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.running.as_ref().map(|r| r.local_addr)
    }

    /// `Degraded(RLY-DS-008)` while the latest beacon could not be sent to every target;
    /// `Ok` otherwise. Receiving keeps working while sending is degraded.
    pub fn health(&self) -> Health {
        match &self.running {
            Some(r) if r.shared.send_failed.load(Ordering::Relaxed) => {
                Health::Degraded(ErrorCode::new(MODULE, 8))
            }
            _ => Health::Ok,
        }
    }
}

impl Discovery for UdpBeaconDiscovery {
    fn start(&mut self) -> Result<(), Error> {
        if self.running.is_some() {
            return Err(Error::AlreadyStarted);
        }
        // Sign once, before anything is opened: the beacon does not change while running.
        let datagram = match &self.announce {
            Some((beacon, signer)) => Some(encode_beacon(beacon, signer.as_ref())?),
            None => None,
        };
        let bind_failed = |e: std::io::Error| Error::BindFailed {
            reason: e.to_string(),
        };
        let interval = self.config.interval.max(Duration::from_millis(1));
        let socket = UdpSocket::bind(self.config.bind).map_err(bind_failed)?;
        socket.set_broadcast(true).map_err(bind_failed)?;
        socket
            .set_read_timeout(Some(interval.min(MAX_POLL)))
            .map_err(bind_failed)?;
        let local_addr = socket.local_addr().map_err(bind_failed)?;

        let shared = Arc::new(Shared {
            stop: AtomicBool::new(false),
            send_failed: AtomicBool::new(false),
            pending: Mutex::new(Vec::new()),
        });
        let worker = Worker {
            socket,
            shared: Arc::clone(&shared),
            verifier: Arc::clone(&self.verifier),
            own_device: self.own_device,
            datagram,
            targets: self.config.targets.clone(),
            interval,
        };
        let thread = std::thread::Builder::new()
            .name("relay-discovery".into())
            .spawn(move || worker.run())
            .map_err(bind_failed)?;
        self.running = Some(Running {
            shared,
            thread,
            local_addr,
        });
        Ok(())
    }

    fn stop(&mut self) -> Result<(), Error> {
        let running = self.running.take().ok_or(Error::NotStarted)?;
        running.shared.stop.store(true, Ordering::Relaxed);
        // The thread only ever exits through the stop flag; a panic there has nothing to add.
        let _ = running.thread.join();
        Ok(())
    }

    fn sightings(&mut self) -> Result<Vec<PeerSighting>, Error> {
        let running = self.running.as_ref().ok_or(Error::NotStarted)?;
        Ok(std::mem::take(&mut *running.shared.pending()))
    }
}

impl Drop for UdpBeaconDiscovery {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Everything the background thread owns.
struct Worker {
    socket: UdpSocket,
    shared: Arc<Shared>,
    verifier: Arc<dyn BeaconVerifier>,
    own_device: Option<DeviceId>,
    datagram: Option<[u8; BEACON_LEN]>,
    targets: Vec<SocketAddr>,
    interval: Duration,
}

impl Worker {
    fn run(self) {
        // Larger than a beacon, so an oversized datagram shows up as a wrong length.
        let mut buf = [0u8; BEACON_LEN + 1];
        let mut next_send = Instant::now();
        while !self.shared.stop.load(Ordering::Relaxed) {
            if let Some(datagram) = &self.datagram {
                if Instant::now() >= next_send {
                    let failed = self
                        .targets
                        .iter()
                        .filter(|t| self.socket.send_to(datagram, t).is_err())
                        .count();
                    self.shared.send_failed.store(failed > 0, Ordering::Relaxed);
                    next_send = Instant::now() + self.interval;
                }
            }
            match self.socket.recv_from(&mut buf) {
                Ok((n, from)) => {
                    let sighting = accept(&buf[..n], from, self.own_device, self.verifier.as_ref());
                    if let Some(sighting) = sighting {
                        queue(&mut self.shared.pending(), sighting);
                    }
                }
                // Nothing arrived within the poll time (Unix says WouldBlock, Windows TimedOut).
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                // Windows reports "connection reset" here after one of our beacons hit a closed
                // port. Not fatal for a listener; pause briefly so a repeating error cannot spin.
                Err(_) => std::thread::sleep(Duration::from_millis(5)),
            }
        }
    }
}

/// Turns one received datagram into a sighting, or `None` if it must be dropped.
///
/// The sighting's address is the datagram's source IP with the session port from the beacon.
fn accept(
    datagram: &[u8],
    from: SocketAddr,
    own_device: Option<DeviceId>,
    verifier: &dyn BeaconVerifier,
) -> Option<PeerSighting> {
    let beacon = decode_beacon(datagram, verifier).ok()?;
    if Some(beacon.host) == own_device {
        return None;
    }
    Some(PeerSighting {
        device: beacon.host,
        addr: SocketAddr::new(from.ip(), beacon.port),
        via: DiscoveryMethod::Beacon,
    })
}

/// Adds `sighting` unless it is already waiting or the queue is full.
fn queue(pending: &mut Vec<PeerSighting>, sighting: PeerSighting) {
    if pending.len() < MAX_PENDING_SIGHTINGS && !pending.contains(&sighting) {
        pending.push(sighting);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{beacon_for, FailingSigner, FakeSigner, FakeVerifier};

    const LOOPBACK: Ipv4Addr = Ipv4Addr::LOCALHOST;

    fn any_local_port() -> SocketAddr {
        SocketAddr::from((LOOPBACK, 0))
    }

    fn listener() -> UdpBeaconDiscovery {
        let config = BeaconConfig {
            bind: any_local_port(),
            targets: vec![],
            interval: Duration::from_millis(20),
        };
        UdpBeaconDiscovery::new(config, Arc::new(FakeVerifier))
    }

    fn announcer(key: u8, session_port: u16, targets: Vec<SocketAddr>) -> UdpBeaconDiscovery {
        let signer = FakeSigner([key; 32]);
        let beacon = beacon_for(&signer, session_port);
        let config = BeaconConfig {
            bind: any_local_port(),
            targets,
            interval: Duration::from_millis(20),
        };
        UdpBeaconDiscovery::new(config, Arc::new(FakeVerifier)).announce(beacon, Arc::new(signer))
    }

    /// Polls `d` until `want` sightings arrived or 5 s passed.
    fn wait_for(d: &mut UdpBeaconDiscovery, want: usize) -> Vec<PeerSighting> {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut seen = Vec::new();
        while seen.len() < want && Instant::now() < deadline {
            for s in d.sightings().unwrap() {
                if !seen.contains(&s) {
                    seen.push(s);
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        seen
    }

    fn signed(key: u8, port: u16) -> [u8; BEACON_LEN] {
        let signer = FakeSigner([key; 32]);
        encode_beacon(&beacon_for(&signer, port), &signer).unwrap()
    }

    // ---- pure parts: no sockets ----

    #[test]
    fn default_config_matches_design() {
        let c = BeaconConfig::default();
        assert_eq!(c.interval, Duration::from_secs(2));
        assert_eq!(c.bind.port(), DEFAULT_BEACON_PORT);
        assert_eq!(c.targets, vec!["255.255.255.255:47470".parse().unwrap()]);
    }

    #[test]
    fn accept_builds_sighting_from_source_ip_and_beacon_port() {
        let from: SocketAddr = "192.168.1.9:50000".parse().unwrap();
        let s = accept(&signed(4, 4100), from, None, &FakeVerifier).unwrap();
        assert_eq!(s.device, DeviceId([4; 32]));
        assert_eq!(s.addr, "192.168.1.9:4100".parse().unwrap());
        assert_eq!(s.via, DiscoveryMethod::Beacon);
    }

    #[test]
    fn accept_drops_own_beacon_junk_and_forgeries() {
        let from: SocketAddr = "192.168.1.9:50000".parse().unwrap();
        let good = signed(4, 4100);
        assert!(accept(&good, from, Some(DeviceId([4; 32])), &FakeVerifier).is_none());
        assert!(accept(&good, from, Some(DeviceId([5; 32])), &FakeVerifier).is_some());
        assert!(accept(b"hello", from, None, &FakeVerifier).is_none());
        assert!(accept(&[], from, None, &FakeVerifier).is_none());
        let mut forged = good;
        forged[38] ^= 0xFF; // claims to be another device
        assert!(accept(&forged, from, None, &FakeVerifier).is_none());
    }

    #[test]
    fn queue_dedupes_and_is_bounded() {
        let sighting = |n: u16| PeerSighting {
            device: DeviceId([1; 32]),
            addr: SocketAddr::from((LOOPBACK, 1 + n)),
            via: DiscoveryMethod::Beacon,
        };
        let mut pending = Vec::new();
        queue(&mut pending, sighting(0));
        queue(&mut pending, sighting(0));
        assert_eq!(pending.len(), 1);
        for n in 0..(MAX_PENDING_SIGHTINGS as u16 + 50) {
            queue(&mut pending, sighting(n));
        }
        assert_eq!(pending.len(), MAX_PENDING_SIGHTINGS);
    }

    // ---- lifecycle ----

    #[test]
    fn lifecycle_errors_and_restart() {
        let mut d = listener();
        assert!(matches!(d.sightings(), Err(Error::NotStarted)));
        assert!(matches!(d.stop(), Err(Error::NotStarted)));
        assert_eq!(d.local_addr(), None);

        d.start().unwrap();
        assert!(matches!(d.start(), Err(Error::AlreadyStarted)));
        assert_ne!(d.local_addr().unwrap().port(), 0);
        assert_eq!(d.sightings().unwrap(), vec![]);
        assert_eq!(d.health(), Health::Ok);

        d.stop().unwrap();
        assert!(matches!(d.stop(), Err(Error::NotStarted)));
        d.start().unwrap();
        d.stop().unwrap();
    }

    #[test]
    fn bind_failure_is_reported() {
        let taken = UdpSocket::bind(any_local_port()).unwrap();
        let mut d = listener();
        d.config.bind = taken.local_addr().unwrap();
        assert!(matches!(d.start(), Err(Error::BindFailed { .. })));
        // A failed start leaves it stopped, and it can start once the port is free.
        assert!(matches!(d.sightings(), Err(Error::NotStarted)));
        drop(taken);
        d.start().unwrap();
    }

    #[test]
    fn sign_failure_stops_start() {
        let beacon = beacon_for(&FakeSigner([1; 32]), 4100);
        let mut d = listener().announce(beacon, Arc::new(FailingSigner));
        assert!(matches!(d.start(), Err(Error::SignFailed { .. })));
        assert!(matches!(d.stop(), Err(Error::NotStarted)));
    }

    // ---- real datagrams over loopback ----

    #[test]
    fn listener_sights_two_announcers() {
        let mut l = listener();
        l.start().unwrap();
        let target = l.local_addr().unwrap();
        let mut a = announcer(0xA1, 4101, vec![target]);
        let mut b = announcer(0xB2, 4102, vec![target]);
        a.start().unwrap();
        b.start().unwrap();

        let mut seen = wait_for(&mut l, 2);
        seen.sort_by_key(|s| s.addr.port());
        assert_eq!(
            seen,
            vec![
                PeerSighting {
                    device: DeviceId([0xA1; 32]),
                    addr: SocketAddr::from((LOOPBACK, 4101)),
                    via: DiscoveryMethod::Beacon,
                },
                PeerSighting {
                    device: DeviceId([0xB2; 32]),
                    addr: SocketAddr::from((LOOPBACK, 4102)),
                    via: DiscoveryMethod::Beacon,
                },
            ]
        );
        assert_eq!(a.health(), Health::Ok);
    }

    #[test]
    fn beacons_repeat_and_stop_when_stopped() {
        let mut l = listener();
        l.start().unwrap();
        let mut a = announcer(0xA1, 4101, vec![l.local_addr().unwrap()]);
        a.start().unwrap();
        // The same peer shows up again after each drain: beacons keep coming.
        assert_eq!(wait_for(&mut l, 1).len(), 1);
        assert_eq!(wait_for(&mut l, 1).len(), 1);

        a.stop().unwrap();
        let _ = l.sightings().unwrap(); // drop anything already queued
        std::thread::sleep(Duration::from_millis(150));
        let _ = l.sightings().unwrap(); // and anything that was in flight
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(l.sightings().unwrap(), vec![]);
    }

    #[test]
    fn two_hosts_find_each_other_and_not_themselves() {
        // Each announces to both sockets, like a broadcast that also loops back to the sender.
        let sock_a = UdpSocket::bind(any_local_port()).unwrap();
        let sock_b = UdpSocket::bind(any_local_port()).unwrap();
        let (addr_a, addr_b) = (sock_a.local_addr().unwrap(), sock_b.local_addr().unwrap());
        drop((sock_a, sock_b));

        let mut a = announcer(0xA1, 4101, vec![addr_a, addr_b]);
        let mut b = announcer(0xB2, 4102, vec![addr_a, addr_b]);
        a.config.bind = addr_a;
        b.config.bind = addr_b;
        a.start().unwrap();
        b.start().unwrap();

        let seen_by_a = wait_for(&mut a, 1);
        let seen_by_b = wait_for(&mut b, 1);
        assert_eq!(seen_by_a.len(), 1);
        assert_eq!(seen_by_a[0].device, DeviceId([0xB2; 32]));
        assert_eq!(seen_by_b.len(), 1);
        assert_eq!(seen_by_b[0].device, DeviceId([0xA1; 32]));
        // Give self-addressed beacons time to arrive; they must never be reported.
        std::thread::sleep(Duration::from_millis(100));
        assert!(a
            .sightings()
            .unwrap()
            .iter()
            .all(|s| s.device != DeviceId([0xA1; 32])));
    }

    #[test]
    fn junk_and_forged_datagrams_do_not_stop_the_listener() {
        let mut l = listener();
        l.start().unwrap();
        let target = l.local_addr().unwrap();
        let raw = UdpSocket::bind(any_local_port()).unwrap();

        let mut forged = signed(0xEE, 4199);
        forged[38] ^= 0xFF;
        raw.send_to(b"not a beacon", target).unwrap();
        raw.send_to(&[0u8; 2_000], target).unwrap();
        raw.send_to(&forged, target).unwrap();
        raw.send_to(&signed(0xC3, 4103), target).unwrap();

        let seen = wait_for(&mut l, 1);
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].device, DeviceId([0xC3; 32]));
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(l.sightings().unwrap(), vec![]);
    }

    #[test]
    fn unsendable_target_degrades_health_but_listening_continues() {
        let mut l = listener();
        l.start().unwrap();
        // An IPv6 target cannot be reached from this IPv4 socket, so every send fails.
        let mut a = announcer(0xA1, 4101, vec!["[::1]:9".parse().unwrap()]);
        a.start().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while a.health() == Health::Ok && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(a.health(), Health::Degraded(ErrorCode::new(MODULE, 8)));

        // It still hears others.
        let raw = UdpSocket::bind(any_local_port()).unwrap();
        raw.send_to(&signed(0xC3, 4103), a.local_addr().unwrap())
            .unwrap();
        assert_eq!(wait_for(&mut a, 1).len(), 1);
        a.stop().unwrap();
        assert_eq!(a.health(), Health::Ok);
    }

    #[test]
    fn drop_stops_the_thread_and_frees_the_port() {
        let mut d = listener();
        d.start().unwrap();
        let addr = d.local_addr().unwrap();
        drop(d);
        UdpSocket::bind(addr).expect("port is free again after drop");
    }
}
