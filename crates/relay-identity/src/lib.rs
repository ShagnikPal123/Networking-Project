//! Relay `identity` module (error prefix `RLY-ID`). Device keys, site keys, certificates, chain checks. No network.
//!
//! Boundaries and rules: see `docs/design/01-architecture.md` and
//! `docs/agent/modules/identity.md`.
//!
//! P0 status: this is the public interface. Only [`Fingerprint`] formatting and parsing are real;
//! key generation, signing and chain checks return [`Error::NotImplemented`].

use std::fmt;

use relay_common::{DeviceId, Epoch, ErrorCode, Module, RelayError, SiteId};

/// This crate's module tag, used in error codes and health reports.
pub const MODULE: Module = Module::Identity;

/// Errors raised by the `identity` module.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The feature exists in the design but is not built yet.
    #[error("{code} not implemented yet: {what}", code = ErrorCode::new(MODULE, 1))]
    NotImplemented {
        /// The missing feature.
        what: &'static str,
    },
    /// A signature did not match the signed bytes and public key.
    #[error("{code} signature is not valid", code = ErrorCode::new(MODULE, 2))]
    BadSignature,
    /// A certificate chain does not lead back to the site key.
    #[error("{code} certificate chain is broken", code = ErrorCode::new(MODULE, 3))]
    ChainBroken,
    /// A certificate is past its expiry time.
    #[error("{code} certificate has expired", code = ErrorCode::new(MODULE, 4))]
    Expired,
    /// A fingerprint string is not in the `XXXX-XXXX` form.
    #[error("{code} not a valid fingerprint", code = ErrorCode::new(MODULE, 5))]
    BadFingerprint,
}

impl RelayError for Error {
    fn code(&self) -> ErrorCode {
        match self {
            Error::NotImplemented { .. } => ErrorCode::new(MODULE, 1),
            Error::BadSignature => ErrorCode::new(MODULE, 2),
            Error::ChainBroken => ErrorCode::new(MODULE, 3),
            Error::Expired => ErrorCode::new(MODULE, 4),
            Error::BadFingerprint => ErrorCode::new(MODULE, 5),
        }
    }
}

/// Result type for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// An Ed25519 public key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PublicKey(pub [u8; 32]);

/// An Ed25519 signature.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Signature(pub [u8; 64]);

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Signature(..)")
    }
}

/// A device's key pair. The secret half is never printed or exposed.
pub struct DeviceKeyPair {
    // Secret bytes stay private to this crate.
    _secret: [u8; 32],
    public: PublicKey,
}

impl DeviceKeyPair {
    /// The public half.
    pub fn public(&self) -> PublicKey {
        self.public
    }
}

impl fmt::Debug for DeviceKeyPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceKeyPair")
            .field("public", &self.public)
            .field("secret", &"<hidden>")
            .finish()
    }
}

/// Symbols for fingerprints: 32 characters with no `0 O 1 I`.
const ALPHABET: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// The short human form of a device ID, like `K7QF-2M9X`.
///
/// It is the first 5 bytes (40 bits) of a [`DeviceId`], written as 8 symbols of 5 bits each,
/// with a dash in the middle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fingerprint([u8; 5]);

impl Fingerprint {
    /// Builds the fingerprint of a device ID.
    pub fn from_device_id(id: &DeviceId) -> Self {
        let mut b = [0u8; 5];
        b.copy_from_slice(&id.0[..5]);
        Fingerprint(b)
    }

    /// Parses `K7QF-2M9X`. Case-insensitive; the dash is optional.
    pub fn parse(s: &str) -> Result<Self> {
        let mut bits: u64 = 0;
        let mut n = 0;
        for c in s.chars().filter(|c| *c != '-') {
            let up = c.to_ascii_uppercase();
            let idx = ALPHABET
                .iter()
                .position(|&a| a as char == up)
                .ok_or(Error::BadFingerprint)?;
            bits = (bits << 5) | idx as u64;
            n += 1;
        }
        if n != 8 {
            return Err(Error::BadFingerprint);
        }
        let mut b = [0u8; 5];
        b.copy_from_slice(&bits.to_be_bytes()[3..]);
        Ok(Fingerprint(b))
    }

    /// True if `id` starts with the bytes this fingerprint was made from.
    pub fn matches(&self, id: &DeviceId) -> bool {
        id.0[..5] == self.0
    }
}

impl fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut bits: u64 = 0;
        for byte in self.0 {
            bits = (bits << 8) | byte as u64;
        }
        for i in 0..8 {
            if i == 4 {
                f.write_str("-")?;
            }
            let idx = (bits >> (35 - 5 * i)) & 31;
            write!(f, "{}", ALPHABET[idx as usize] as char)?;
        }
        Ok(())
    }
}

/// Signed by the site key: names the current host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostCertificate {
    /// The site this certificate belongs to.
    pub site: SiteId,
    /// The device allowed to act as host.
    pub host: DeviceId,
    /// Host-change counter; the higher epoch wins.
    pub epoch: Epoch,
    /// Expiry as seconds since the Unix epoch.
    pub expiry: u64,
}

/// Signed by the site key: the owner's approval, given in advance, for one device to take over
/// from one specific host at one specific epoch. Useless after any other handoff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeputyCertificate {
    /// The site this certificate belongs to.
    pub site: SiteId,
    /// The device allowed to take over.
    pub deputy: DeviceId,
    /// The host this deputy may replace.
    pub replaces_host: DeviceId,
    /// The epoch of the host being replaced; the deputy becomes host at the next epoch.
    pub replaces_epoch: Epoch,
    /// Position in the owner's list; lower wins if two deputies step up.
    pub priority: u8,
}

/// Signed by the host key: says a device is an approved member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberCertificate {
    /// The site this certificate belongs to.
    pub site: SiteId,
    /// The approved device.
    pub device: DeviceId,
    /// Display name chosen for the device.
    pub name: String,
    /// The host epoch this certificate was issued under.
    pub epoch: Epoch,
    /// Expiry as seconds since the Unix epoch.
    pub expiry: u64,
}

/// A certificate body with the key that signed it and the signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signed<T> {
    /// What was signed.
    pub body: T,
    /// The signer's public key.
    pub signer: PublicKey,
    /// The signature over the encoded body.
    pub signature: Signature,
}

/// Makes a new device key pair. Not built yet.
pub fn generate() -> Result<DeviceKeyPair> {
    Err(Error::NotImplemented {
        what: "key generation",
    })
}

/// Hashes a public key into a [`DeviceId`]. Not built yet.
pub fn device_id(_key: &PublicKey) -> Result<DeviceId> {
    Err(Error::NotImplemented {
        what: "device id hashing",
    })
}

/// Signs `message` with the device key. Not built yet.
pub fn sign(_key: &DeviceKeyPair, _message: &[u8]) -> Result<Signature> {
    Err(Error::NotImplemented { what: "signing" })
}

/// Checks `signature` over `message` against `key`. Not built yet.
pub fn verify(_key: &PublicKey, _message: &[u8], _signature: &Signature) -> Result<()> {
    Err(Error::NotImplemented {
        what: "signature check",
    })
}

/// Checks site key -> host certificate -> member certificate at time `now` (Unix seconds).
/// Not built yet.
pub fn verify_chain(
    _site_key: &PublicKey,
    _host: &Signed<HostCertificate>,
    _member: &Signed<MemberCertificate>,
    _now: u64,
) -> Result<()> {
    Err(Error::NotImplemented {
        what: "chain verification",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_error_code_has_module_prefix() {
        let all = [
            (Error::NotImplemented { what: "x" }, "RLY-ID-001"),
            (Error::BadSignature, "RLY-ID-002"),
            (Error::ChainBroken, "RLY-ID-003"),
            (Error::Expired, "RLY-ID-004"),
            (Error::BadFingerprint, "RLY-ID-005"),
        ];
        for (e, code) in all {
            assert_eq!(e.code().to_string(), code);
            assert!(e.to_string().starts_with(code), "{e}");
        }
    }

    #[test]
    fn fingerprint_known_value() {
        let mut id = [0u8; 32];
        id[..5].copy_from_slice(&[0, 0, 0, 0, 0]);
        assert_eq!(
            Fingerprint::from_device_id(&DeviceId(id)).to_string(),
            "AAAA-AAAA"
        );
        id[..5].copy_from_slice(&[0xFF; 5]);
        assert_eq!(
            Fingerprint::from_device_id(&DeviceId(id)).to_string(),
            "9999-9999"
        );
    }

    #[test]
    fn fingerprint_round_trip_and_lenient_parse() {
        let mut id = [7u8; 32];
        id[..5].copy_from_slice(&[0x12, 0x34, 0x56, 0x78, 0x9A]);
        let id = DeviceId(id);
        let fp = Fingerprint::from_device_id(&id);
        let s = fp.to_string();
        assert_eq!(s.len(), 9);
        assert_eq!(s.as_bytes()[4], b'-');
        assert_eq!(Fingerprint::parse(&s).unwrap(), fp);
        assert_eq!(Fingerprint::parse(&s.to_lowercase()).unwrap(), fp);
        assert_eq!(Fingerprint::parse(&s.replace('-', "")).unwrap(), fp);
        assert!(fp.matches(&id));
        assert!(!fp.matches(&DeviceId([0; 32])));
    }

    #[test]
    fn fingerprint_rejects_bad_input() {
        for bad in [
            "",
            "ABCD",
            "ABCD-EFGHJ",
            "ABCD-EFG0",
            "ABCD-EFGI",
            "ABCD-EFG!",
        ] {
            assert!(matches!(
                Fingerprint::parse(bad),
                Err(Error::BadFingerprint)
            ));
        }
    }

    #[test]
    fn debug_never_shows_secret() {
        let kp = DeviceKeyPair {
            _secret: [0xAB; 32],
            public: PublicKey([1; 32]),
        };
        let s = format!("{kp:?}");
        assert!(s.contains("<hidden>"));
        assert!(!s.contains("171"));
    }

    #[test]
    fn stubs_return_not_implemented() {
        assert!(matches!(generate(), Err(Error::NotImplemented { .. })));
        assert!(matches!(
            device_id(&PublicKey([0; 32])),
            Err(Error::NotImplemented { .. })
        ));
    }
}
