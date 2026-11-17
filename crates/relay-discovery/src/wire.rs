//! Beacon wire format and signing hooks. Pure: no sockets, no clock.
//!
//! # Datagram layout
//!
//! All integers are big-endian. One beacon is exactly [`BEACON_LEN`] bytes.
//!
//! ```text
//! offset  size  field
//!      0     4  magic "RLYB"
//!      4     2  version (currently 1)
//!      6    32  site ID
//!     38    32  host device ID
//!     70     8  epoch
//!     78     2  session port
//!     80    32  signer's public key
//!    112    64  signature over bytes 0..112
//! ```
//!
//! The signature covers the magic and version too, so a beacon cannot be replayed as another
//! message type or version.
//!
//! # What the signature does and does not give
//!
//! It proves the beacon was written by the holder of the key behind the host device ID. It does
//! **not** make the sender trusted, and a captured beacon can be re-sent by anyone on the LAN.
//! That is fine: discovery only says "something may be here"; the session handshake decides.

use crate::{Beacon, Error};
use relay_common::{DeviceId, Epoch, SiteId};

/// First four bytes of every beacon.
pub const BEACON_MAGIC: [u8; 4] = *b"RLYB";
/// The beacon wire version written and read by this build.
pub const BEACON_VERSION: u16 = 1;
/// Exact size of a beacon datagram in bytes.
pub const BEACON_LEN: usize = 176;
/// How many leading bytes the signature covers.
const SIGNED_LEN: usize = 112;

/// Signs this device's beacon. Implemented above discovery (by whoever holds the device key),
/// so this crate never sees a secret key.
pub trait BeaconSigner: Send + Sync {
    /// The public key that matches the signing key.
    fn public_key(&self) -> [u8; 32];
    /// Signs `message`. An error text is reported as `RLY-DS-009`.
    fn sign(&self, message: &[u8]) -> Result<[u8; 64], String>;
}

/// Checks a received beacon. Implemented above discovery.
pub trait BeaconVerifier: Send + Sync {
    /// Must return true only if `public_key` is the key of device `host` **and** `signature`
    /// is a valid signature by that key over `message`.
    fn verify(
        &self,
        host: &DeviceId,
        public_key: &[u8; 32],
        message: &[u8],
        signature: &[u8; 64],
    ) -> bool;
}

/// Serialises and signs `beacon`.
pub fn encode_beacon(
    beacon: &Beacon,
    signer: &dyn BeaconSigner,
) -> Result<[u8; BEACON_LEN], Error> {
    if beacon.port == 0 {
        return Err(Error::MalformedBeacon {
            reason: "port is zero",
        });
    }
    let mut out = [0u8; BEACON_LEN];
    out[0..4].copy_from_slice(&BEACON_MAGIC);
    out[4..6].copy_from_slice(&BEACON_VERSION.to_be_bytes());
    out[6..38].copy_from_slice(&beacon.site.0);
    out[38..70].copy_from_slice(&beacon.host.0);
    out[70..78].copy_from_slice(&beacon.epoch.0.to_be_bytes());
    out[78..80].copy_from_slice(&beacon.port.to_be_bytes());
    out[80..112].copy_from_slice(&signer.public_key());
    let signature = signer
        .sign(&out[..SIGNED_LEN])
        .map_err(|reason| Error::SignFailed { reason })?;
    out[SIGNED_LEN..].copy_from_slice(&signature);
    Ok(out)
}

/// Parses a datagram and checks its signature. Anything that fails here must be dropped.
pub fn decode_beacon(bytes: &[u8], verifier: &dyn BeaconVerifier) -> Result<Beacon, Error> {
    if bytes.len() < 6 || bytes[0..4] != BEACON_MAGIC {
        return Err(Error::MalformedBeacon {
            reason: "not a Relay beacon",
        });
    }
    let version = u16::from_be_bytes([bytes[4], bytes[5]]);
    if version != BEACON_VERSION {
        return Err(Error::UnsupportedVersion { version });
    }
    if bytes.len() != BEACON_LEN {
        return Err(Error::MalformedBeacon {
            reason: "wrong length",
        });
    }
    let field = |from: usize| -> [u8; 32] {
        let mut b = [0u8; 32];
        b.copy_from_slice(&bytes[from..from + 32]);
        b
    };
    let mut epoch = [0u8; 8];
    epoch.copy_from_slice(&bytes[70..78]);
    let beacon = Beacon {
        site: SiteId(field(6)),
        host: DeviceId(field(38)),
        epoch: Epoch(u64::from_be_bytes(epoch)),
        port: u16::from_be_bytes([bytes[78], bytes[79]]),
    };
    if beacon.port == 0 {
        return Err(Error::MalformedBeacon {
            reason: "port is zero",
        });
    }
    let mut signature = [0u8; 64];
    signature.copy_from_slice(&bytes[SIGNED_LEN..]);
    if !verifier.verify(&beacon.host, &field(80), &bytes[..SIGNED_LEN], &signature) {
        return Err(Error::BadSignature);
    }
    Ok(beacon)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{beacon_for, FailingSigner, FakeSigner, FakeVerifier};

    #[test]
    fn round_trip() {
        let signer = FakeSigner([3; 32]);
        let beacon = beacon_for(&signer, 4100);
        let bytes = encode_beacon(&beacon, &signer).unwrap();
        assert_eq!(decode_beacon(&bytes, &FakeVerifier).unwrap(), beacon);
    }

    #[test]
    fn layout_is_as_documented() {
        let signer = FakeSigner([3; 32]);
        let beacon = Beacon {
            site: SiteId([0xAA; 32]),
            host: DeviceId([3; 32]),
            epoch: Epoch(0x0102_0304_0506_0708),
            port: 0x1234,
        };
        let b = encode_beacon(&beacon, &signer).unwrap();
        assert_eq!(b.len(), 176);
        assert_eq!(&b[0..6], b"RLYB\x00\x01");
        assert_eq!(b[6..38], [0xAA; 32]);
        assert_eq!(b[38..70], [3; 32]);
        assert_eq!(b[70..78], [1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(b[78..80], [0x12, 0x34]);
        assert_eq!(b[80..112], [3; 32]);
    }

    #[test]
    fn every_flipped_byte_is_rejected() {
        let signer = FakeSigner([9; 32]);
        let good = encode_beacon(&beacon_for(&signer, 4100), &signer).unwrap();
        for i in 0..BEACON_LEN {
            let mut bad = good;
            bad[i] ^= 0x01;
            assert!(decode_beacon(&bad, &FakeVerifier).is_err(), "byte {i}");
        }
    }

    #[test]
    fn wrong_key_for_host_is_rejected() {
        // Signed correctly, but by a key that is not the named host's key.
        let signer = FakeSigner([9; 32]);
        let mut beacon = beacon_for(&signer, 4100);
        beacon.host = DeviceId([8; 32]);
        let bytes = encode_beacon(&beacon, &signer).unwrap();
        assert!(matches!(
            decode_beacon(&bytes, &FakeVerifier),
            Err(Error::BadSignature)
        ));
    }

    #[test]
    fn malformed_inputs() {
        let signer = FakeSigner([9; 32]);
        let good = encode_beacon(&beacon_for(&signer, 4100), &signer).unwrap();
        let malformed = |b: &[u8]| {
            matches!(
                decode_beacon(b, &FakeVerifier),
                Err(Error::MalformedBeacon { .. })
            )
        };
        assert!(malformed(&[]));
        assert!(malformed(b"RLYB"));
        assert!(malformed(&[0u8; BEACON_LEN]));
        assert!(malformed(&good[..BEACON_LEN - 1]));
        let mut long = good.to_vec();
        long.push(0);
        assert!(malformed(&long));

        let mut newer = good;
        newer[5] = 2;
        assert!(matches!(
            decode_beacon(&newer, &FakeVerifier),
            Err(Error::UnsupportedVersion { version: 2 })
        ));
    }

    #[test]
    fn zero_port_is_refused_both_ways() {
        let signer = FakeSigner([9; 32]);
        let zero = beacon_for(&signer, 0);
        assert!(matches!(
            encode_beacon(&zero, &signer),
            Err(Error::MalformedBeacon { .. })
        ));
        // A correctly signed datagram that carries port 0 is still refused.
        let mut bytes = encode_beacon(&beacon_for(&signer, 7), &signer).unwrap();
        bytes[78..80].copy_from_slice(&[0, 0]);
        let sig = signer.sign(&bytes[..SIGNED_LEN]).unwrap();
        bytes[SIGNED_LEN..].copy_from_slice(&sig);
        assert!(matches!(
            decode_beacon(&bytes, &FakeVerifier),
            Err(Error::MalformedBeacon { .. })
        ));
    }

    #[test]
    fn signer_failure_surfaces_as_sign_failed() {
        let beacon = beacon_for(&FakeSigner([1; 32]), 4100);
        assert!(matches!(
            encode_beacon(&beacon, &FailingSigner),
            Err(Error::SignFailed { .. })
        ));
    }
}
