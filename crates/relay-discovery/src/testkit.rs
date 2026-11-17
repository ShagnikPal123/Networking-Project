//! Test-only stand-ins for the real signer and verifier. Not secure; never built outside tests.

use crate::wire::{BeaconSigner, BeaconVerifier};
use crate::Beacon;
use relay_common::{DeviceId, Epoch, SiteId};

/// "Signs" with a keyed checksum. The fake device ID of a key is the key itself.
pub(crate) struct FakeSigner(pub [u8; 32]);

fn fake_signature(key: &[u8; 32], message: &[u8]) -> [u8; 64] {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in key.iter().chain(message) {
        h = (h ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    let mut out = [0u8; 64];
    for (i, o) in out.iter_mut().enumerate() {
        *o = (h.rotate_left(i as u32) as u8) ^ key[i % 32];
    }
    out
}

impl BeaconSigner for FakeSigner {
    fn public_key(&self) -> [u8; 32] {
        self.0
    }
    fn sign(&self, message: &[u8]) -> Result<[u8; 64], String> {
        Ok(fake_signature(&self.0, message))
    }
}

/// A signer that always refuses.
pub(crate) struct FailingSigner;

impl BeaconSigner for FailingSigner {
    fn public_key(&self) -> [u8; 32] {
        [0; 32]
    }
    fn sign(&self, _message: &[u8]) -> Result<[u8; 64], String> {
        Err("key is locked".into())
    }
}

/// Accepts what [`FakeSigner`] produces, and only for the device whose ID equals the key.
pub(crate) struct FakeVerifier;

impl BeaconVerifier for FakeVerifier {
    fn verify(&self, host: &DeviceId, key: &[u8; 32], message: &[u8], sig: &[u8; 64]) -> bool {
        host.0 == *key && fake_signature(key, message) == *sig
    }
}

/// A beacon announcing the device that `signer` stands for.
pub(crate) fn beacon_for(signer: &FakeSigner, port: u16) -> Beacon {
    Beacon {
        site: SiteId([0x51; 32]),
        host: DeviceId(signer.0),
        epoch: Epoch(1),
        port,
    }
}
