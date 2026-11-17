//! Relay `identity` module (error prefix `RLY-ID`). Device keys, site keys, certificates, chain checks. No network.
//!
//! Boundaries and rules: see `docs/design/01-architecture.md` and
//! `docs/agent/modules/identity.md`.
//!
//! Everything here is real: Ed25519 keys (`ed25519-dalek`, strict verification), device and site
//! IDs (domain-separated SHA-256 of the public key), certificate signing, and the offline chain
//! check `site key -> host certificate -> member certificate`.
//!
//! This crate does no I/O. To keep a key across restarts, the caller saves
//! [`DeviceKeyPair::secret_bytes`] through the `store` module and loads it back with
//! [`DeviceKeyPair::from_secret_bytes`].
//!
//! # What gets signed
//!
//! A signature never covers bare struct memory. Each certificate has one fixed byte encoding
//! (see [`HostCertificate::signing_bytes`] and [`MemberCertificate::signing_bytes`]) that starts
//! with a tag naming the certificate kind and version, so a signature made for one kind can never
//! be replayed as another. All integers are big-endian.

use std::fmt;

use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use relay_common::{DeviceId, Epoch, ErrorCode, Module, RelayError, SiteId};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

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
    /// The 32 bytes are not a usable Ed25519 public key.
    #[error("{code} not a valid public key", code = ErrorCode::new(MODULE, 6))]
    BadPublicKey,
    /// The operating system could not supply random bytes for a new key.
    #[error("{code} system random source failed", code = ErrorCode::new(MODULE, 7))]
    RandomSource,
}

impl RelayError for Error {
    fn code(&self) -> ErrorCode {
        match self {
            Error::NotImplemented { .. } => ErrorCode::new(MODULE, 1),
            Error::BadSignature => ErrorCode::new(MODULE, 2),
            Error::ChainBroken => ErrorCode::new(MODULE, 3),
            Error::Expired => ErrorCode::new(MODULE, 4),
            Error::BadFingerprint => ErrorCode::new(MODULE, 5),
            Error::BadPublicKey => ErrorCode::new(MODULE, 6),
            Error::RandomSource => ErrorCode::new(MODULE, 7),
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

/// An Ed25519 key pair. The secret half is never printed and is wiped from memory on drop.
///
/// The same type serves as a device key and as a site key (see [`SiteKeyPair`]).
pub struct DeviceKeyPair {
    // `SigningKey` zeroes itself on drop (the `zeroize` feature of `ed25519-dalek`).
    secret: SigningKey,
    public: PublicKey,
}

/// A site's key pair. Whoever holds it is the owner. Same shape as a device key.
pub type SiteKeyPair = DeviceKeyPair;

impl DeviceKeyPair {
    /// The public half.
    pub fn public(&self) -> PublicKey {
        self.public
    }

    /// Rebuilds a key pair from the 32 secret bytes saved earlier.
    pub fn from_secret_bytes(secret: &[u8; 32]) -> Self {
        let secret = SigningKey::from_bytes(secret);
        let public = PublicKey(secret.verifying_key().to_bytes());
        Self { secret, public }
    }

    /// The 32 secret bytes, for saving to disk. The returned buffer wipes itself on drop.
    ///
    /// Handle with care: anyone holding these bytes can act as this device. Never log them or
    /// send them over the network.
    pub fn secret_bytes(&self) -> Zeroizing<[u8; 32]> {
        Zeroizing::new(self.secret.to_bytes())
    }

    /// The device ID of the public half.
    pub fn device_id(&self) -> DeviceId {
        DeviceId(hash_key(DEVICE_ID_TAG, &self.public))
    }

    /// The site ID of the public half, when this pair is used as a site key.
    pub fn site_id(&self) -> SiteId {
        site_id(&self.public)
    }

    /// Signs `message`. Ed25519 signing is deterministic and cannot fail.
    pub fn sign(&self, message: &[u8]) -> Signature {
        Signature(self.secret.sign(message).to_bytes())
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

/// Domain tag for device IDs.
const DEVICE_ID_TAG: &[u8] = b"relay/device-id/v1";
/// Domain tag for site IDs.
const SITE_ID_TAG: &[u8] = b"relay/site-id/v1";
/// Domain tag at the start of the signed bytes of a host certificate.
const HOST_CERT_TAG: &[u8] = b"relay/host-cert/v1";
/// Domain tag at the start of the signed bytes of a member certificate.
const MEMBER_CERT_TAG: &[u8] = b"relay/member-cert/v1";

/// SHA-256 over `tag`, a zero byte, then the key. The tag keeps device and site IDs apart even
/// when one key is used for both.
fn hash_key(tag: &[u8], key: &PublicKey) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(tag);
    h.update([0u8]);
    h.update(key.0);
    h.finalize().into()
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

/// Signed by the site key: names the current host and its deputies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostCertificate {
    /// The site this certificate belongs to.
    pub site: SiteId,
    /// The device allowed to act as host.
    pub host: DeviceId,
    /// Host-change counter; the higher epoch wins.
    pub epoch: Epoch,
    /// Devices the owner pre-approved to take over, in priority order.
    pub deputies: Vec<DeviceId>,
    /// Expiry as seconds since the Unix epoch.
    pub expiry: u64,
}

impl HostCertificate {
    /// The exact bytes a site key signs.
    ///
    /// `tag | 0 | site (32) | host (32) | epoch (u64) | deputy count (u32) | deputies (32 each) |
    /// expiry (u64)`.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HOST_CERT_TAG.len() + 85 + 32 * self.deputies.len());
        out.extend_from_slice(HOST_CERT_TAG);
        out.push(0);
        out.extend_from_slice(&self.site.0);
        out.extend_from_slice(&self.host.0);
        out.extend_from_slice(&self.epoch.0.to_be_bytes());
        out.extend_from_slice(&(self.deputies.len() as u32).to_be_bytes());
        for d in &self.deputies {
            out.extend_from_slice(&d.0);
        }
        out.extend_from_slice(&self.expiry.to_be_bytes());
        out
    }
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

impl MemberCertificate {
    /// The exact bytes a host key signs.
    ///
    /// `tag | 0 | site (32) | device (32) | name length (u32) | name (UTF-8) | epoch (u64) |
    /// expiry (u64)`.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(MEMBER_CERT_TAG.len() + 85 + self.name.len());
        out.extend_from_slice(MEMBER_CERT_TAG);
        out.push(0);
        out.extend_from_slice(&self.site.0);
        out.extend_from_slice(&self.device.0);
        out.extend_from_slice(&(self.name.len() as u32).to_be_bytes());
        out.extend_from_slice(self.name.as_bytes());
        out.extend_from_slice(&self.epoch.0.to_be_bytes());
        out.extend_from_slice(&self.expiry.to_be_bytes());
        out
    }
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

/// Makes a new key pair from the operating system's random source.
///
/// Fails with [`Error::RandomSource`] if the system cannot supply random bytes.
pub fn generate() -> Result<DeviceKeyPair> {
    let mut seed = Zeroizing::new([0u8; 32]);
    getrandom::getrandom(seed.as_mut()).map_err(|_| Error::RandomSource)?;
    Ok(DeviceKeyPair::from_secret_bytes(&seed))
}

/// Hashes a public key into a [`DeviceId`].
///
/// Fails with [`Error::BadPublicKey`] if the bytes are not a usable Ed25519 key, so an ID is
/// never handed out for a key that could not sign anything.
pub fn device_id(key: &PublicKey) -> Result<DeviceId> {
    parse_key(key)?;
    Ok(DeviceId(hash_key(DEVICE_ID_TAG, key)))
}

/// Hashes a site public key into a [`SiteId`].
pub fn site_id(key: &PublicKey) -> SiteId {
    SiteId(hash_key(SITE_ID_TAG, key))
}

/// Signs `message` with the key pair.
pub fn sign(key: &DeviceKeyPair, message: &[u8]) -> Result<Signature> {
    Ok(key.sign(message))
}

/// Checks `signature` over `message` against `key`.
///
/// Uses strict verification: weak keys and non-canonical signatures are refused.
/// Fails with [`Error::BadPublicKey`] or [`Error::BadSignature`].
pub fn verify(key: &PublicKey, message: &[u8], signature: &Signature) -> Result<()> {
    let vk = parse_key(key)?;
    let sig = ed25519_dalek::Signature::from_bytes(&signature.0);
    vk.verify_strict(message, &sig)
        .map_err(|_| Error::BadSignature)
}

fn parse_key(key: &PublicKey) -> Result<VerifyingKey> {
    let vk = VerifyingKey::from_bytes(&key.0).map_err(|_| Error::BadPublicKey)?;
    if vk.is_weak() {
        return Err(Error::BadPublicKey);
    }
    Ok(vk)
}

/// Signs a host certificate with the site key. Done by the owner.
pub fn sign_host_certificate(
    site_key: &SiteKeyPair,
    body: HostCertificate,
) -> Signed<HostCertificate> {
    let signature = site_key.sign(&body.signing_bytes());
    Signed {
        body,
        signer: site_key.public(),
        signature,
    }
}

/// Signs a member certificate with the host's device key. Done by the host.
pub fn sign_member_certificate(
    host_key: &DeviceKeyPair,
    body: MemberCertificate,
) -> Signed<MemberCertificate> {
    let signature = host_key.sign(&body.signing_bytes());
    Signed {
        body,
        signer: host_key.public(),
        signature,
    }
}

/// Checks that `host` was signed by `site_key`, is for that site, and has not expired at `now`
/// (Unix seconds). A certificate is valid while `now < expiry`.
pub fn verify_host_certificate(
    site_key: &PublicKey,
    host: &Signed<HostCertificate>,
    now: u64,
) -> Result<()> {
    if host.signer != *site_key || host.body.site != site_id(site_key) {
        return Err(Error::ChainBroken);
    }
    verify(site_key, &host.body.signing_bytes(), &host.signature)?;
    if now >= host.body.expiry {
        return Err(Error::Expired);
    }
    Ok(())
}

/// Checks site key -> host certificate -> member certificate at time `now` (Unix seconds).
///
/// Passes only if all of these hold:
/// 1. the host certificate is signed by `site_key` and names that site;
/// 2. the member certificate is signed by the device the host certificate names as host;
/// 3. both certificates name the same site and the same epoch;
/// 4. neither has expired (`now < expiry`).
///
/// Structure and signatures are checked before expiry, so [`Error::Expired`] always means "this
/// was a genuine chain that is now too old".
pub fn verify_chain(
    site_key: &PublicKey,
    host: &Signed<HostCertificate>,
    member: &Signed<MemberCertificate>,
    now: u64,
) -> Result<()> {
    if host.signer != *site_key || host.body.site != site_id(site_key) {
        return Err(Error::ChainBroken);
    }
    verify(site_key, &host.body.signing_bytes(), &host.signature)?;

    let signer_is_host = device_id(&member.signer).map(|id| id == host.body.host);
    if !matches!(signer_is_host, Ok(true))
        || member.body.site != host.body.site
        || member.body.epoch != host.body.epoch
    {
        return Err(Error::ChainBroken);
    }
    verify(
        &member.signer,
        &member.body.signing_bytes(),
        &member.signature,
    )?;

    if now >= host.body.expiry || now >= member.body.expiry {
        return Err(Error::Expired);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex<const N: usize>(s: &str) -> [u8; N] {
        let mut out = [0u8; N];
        assert_eq!(s.len(), N * 2);
        for (i, b) in out.iter_mut().enumerate() {
            *b = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap();
        }
        out
    }

    /// Fixed keys so tests are repeatable.
    fn key(n: u8) -> DeviceKeyPair {
        DeviceKeyPair::from_secret_bytes(&[n; 32])
    }

    struct Chain {
        site: SiteKeyPair,
        host_key: DeviceKeyPair,
        host: Signed<HostCertificate>,
        member: Signed<MemberCertificate>,
    }

    fn chain() -> Chain {
        let site = key(1);
        let host_key = key(2);
        let member_key = key(3);
        let host = sign_host_certificate(
            &site,
            HostCertificate {
                site: site.site_id(),
                host: host_key.device_id(),
                epoch: Epoch(4),
                deputies: vec![key(9).device_id()],
                expiry: 2_000,
            },
        );
        let member = sign_member_certificate(
            &host_key,
            MemberCertificate {
                site: site.site_id(),
                device: member_key.device_id(),
                name: "Laptop".into(),
                epoch: Epoch(4),
                expiry: 1_500,
            },
        );
        Chain {
            site,
            host_key,
            host,
            member,
        }
    }

    #[test]
    fn every_error_code_has_module_prefix() {
        let all = [
            (Error::NotImplemented { what: "x" }, "RLY-ID-001"),
            (Error::BadSignature, "RLY-ID-002"),
            (Error::ChainBroken, "RLY-ID-003"),
            (Error::Expired, "RLY-ID-004"),
            (Error::BadFingerprint, "RLY-ID-005"),
            (Error::BadPublicKey, "RLY-ID-006"),
            (Error::RandomSource, "RLY-ID-007"),
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
        let kp = DeviceKeyPair::from_secret_bytes(&[0xAB; 32]);
        let s = format!("{kp:?}");
        assert!(s.contains("<hidden>"));
        assert!(!s.contains("171, 171"));
        assert!(!s.to_lowercase().contains("abab"));
    }

    /// RFC 8032 section 7.1, test 1 (empty message).
    #[test]
    fn rfc8032_test_vector_1() {
        let kp = DeviceKeyPair::from_secret_bytes(&hex(
            "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
        ));
        assert_eq!(
            kp.public().0,
            hex::<32>("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a")
        );
        let sig = sign(&kp, b"").unwrap();
        assert_eq!(
            sig.0,
            hex::<64>(
                "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bac\
                 c61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"
            )
        );
        verify(&kp.public(), b"", &sig).unwrap();
    }

    /// RFC 8032 section 7.1, test 2 (one-byte message).
    #[test]
    fn rfc8032_test_vector_2() {
        let kp = DeviceKeyPair::from_secret_bytes(&hex(
            "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb",
        ));
        assert_eq!(
            kp.public().0,
            hex::<32>("3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c")
        );
        let sig = sign(&kp, &[0x72]).unwrap();
        assert_eq!(
            sig.0,
            hex::<64>(
                "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e\
                 458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00"
            )
        );
    }

    #[test]
    fn generate_makes_distinct_working_keys() {
        let a = generate().unwrap();
        let b = generate().unwrap();
        assert_ne!(a.public(), b.public());
        assert_ne!(a.device_id(), b.device_id());
        let sig = sign(&a, b"hello").unwrap();
        verify(&a.public(), b"hello", &sig).unwrap();
        assert!(matches!(
            verify(&b.public(), b"hello", &sig),
            Err(Error::BadSignature)
        ));
    }

    #[test]
    fn secret_bytes_round_trip_keeps_identity() {
        let a = generate().unwrap();
        let b = DeviceKeyPair::from_secret_bytes(&a.secret_bytes());
        assert_eq!(a.public(), b.public());
        assert_eq!(a.device_id(), b.device_id());
        assert_eq!(a.sign(b"m"), b.sign(b"m"));
    }

    #[test]
    fn tampered_message_or_signature_fails() {
        let kp = key(5);
        let sig = kp.sign(b"pay 10");
        assert!(matches!(
            verify(&kp.public(), b"pay 99", &sig),
            Err(Error::BadSignature)
        ));
        for i in [0, 31, 32, 63] {
            let mut bad = sig;
            bad.0[i] ^= 1;
            assert!(
                matches!(
                    verify(&kp.public(), b"pay 10", &bad),
                    Err(Error::BadSignature)
                ),
                "byte {i}"
            );
        }
    }

    #[test]
    fn bad_public_keys_are_refused() {
        let sig = key(5).sign(b"m");
        // All zero bytes decode to a small-order point: a weak key.
        let weak = PublicKey([0; 32]);
        assert!(matches!(device_id(&weak), Err(Error::BadPublicKey)));
        assert!(matches!(
            verify(&weak, b"m", &sig),
            Err(Error::BadPublicKey)
        ));
        // Find bytes that are not a curve point at all.
        let off_curve = (0u8..=255)
            .map(|n| {
                let mut b = [0u8; 32];
                b[0] = n;
                b[1] = 0x55;
                PublicKey(b)
            })
            .find(|k| VerifyingKey::from_bytes(&k.0).is_err())
            .expect("some candidate is off the curve");
        assert!(matches!(device_id(&off_curve), Err(Error::BadPublicKey)));
    }

    #[test]
    fn ids_are_stable_and_domain_separated() {
        let kp = key(1);
        assert_eq!(device_id(&kp.public()).unwrap(), kp.device_id());
        assert_eq!(site_id(&kp.public()), kp.site_id());
        // Same key, different purpose, different ID.
        assert_ne!(kp.device_id().0, kp.site_id().0);
        assert_ne!(kp.device_id(), key(2).device_id());
        // Pinned value: changing the ID scheme must be a deliberate, visible change.
        let pinned = DeviceKeyPair::from_secret_bytes(&hex(
            "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
        ));
        let mut h = Sha256::new();
        h.update(b"relay/device-id/v1\0");
        h.update(pinned.public().0);
        let expect: [u8; 32] = h.finalize().into();
        assert_eq!(pinned.device_id().0, expect);
        let fp = Fingerprint::from_device_id(&pinned.device_id());
        assert!(fp.matches(&pinned.device_id()));
    }

    #[test]
    fn signing_bytes_layout() {
        let c = chain();
        let h = c.host.body.signing_bytes();
        assert!(h.starts_with(b"relay/host-cert/v1\0"));
        assert_eq!(h.len(), 19 + 32 + 32 + 8 + 4 + 32 + 8);
        let m = c.member.body.signing_bytes();
        assert!(m.starts_with(b"relay/member-cert/v1\0"));
        assert_eq!(m.len(), 21 + 32 + 32 + 4 + 6 + 8 + 8);
        assert_eq!(&m[m.len() - 8..], &1_500u64.to_be_bytes());
    }

    #[test]
    fn good_chain_passes() {
        let c = chain();
        verify_host_certificate(&c.site.public(), &c.host, 1_000).unwrap();
        verify_chain(&c.site.public(), &c.host, &c.member, 1_000).unwrap();
    }

    #[test]
    fn expiry_is_exclusive() {
        let c = chain();
        let site = c.site.public();
        verify_chain(&site, &c.host, &c.member, 1_499).unwrap();
        // Member certificate ends at 1500, host at 2000.
        assert!(matches!(
            verify_chain(&site, &c.host, &c.member, 1_500),
            Err(Error::Expired)
        ));
        verify_host_certificate(&site, &c.host, 1_999).unwrap();
        assert!(matches!(
            verify_host_certificate(&site, &c.host, 2_000),
            Err(Error::Expired)
        ));
    }

    #[test]
    fn every_edited_field_breaks_the_signature() {
        let c = chain();
        let site = c.site.public();

        let mut edits: Vec<Signed<HostCertificate>> = Vec::new();
        let mut h = c.host.clone();
        h.body.epoch = Epoch(5);
        edits.push(h);
        let mut h = c.host.clone();
        h.body.deputies.push(key(8).device_id());
        edits.push(h);
        let mut h = c.host.clone();
        h.body.deputies.clear();
        edits.push(h);
        let mut h = c.host.clone();
        h.body.expiry = u64::MAX;
        edits.push(h);
        let mut h = c.host.clone();
        h.body.host = key(8).device_id();
        edits.push(h);
        for h in edits {
            assert!(matches!(
                verify_host_certificate(&site, &h, 1_000),
                Err(Error::BadSignature)
            ));
        }

        let mut m = c.member.clone();
        m.body.name = "Laptop2".into();
        assert!(matches!(
            verify_chain(&site, &c.host, &m, 1_000),
            Err(Error::BadSignature)
        ));
        let mut m = c.member.clone();
        m.body.device = key(8).device_id();
        assert!(matches!(
            verify_chain(&site, &c.host, &m, 1_000),
            Err(Error::BadSignature)
        ));
        // A stretched expiry is caught as a bad signature, not accepted.
        let mut m = c.member.clone();
        m.body.expiry = u64::MAX;
        assert!(matches!(
            verify_chain(&site, &c.host, &m, 1_800),
            Err(Error::BadSignature)
        ));
    }

    #[test]
    fn wrong_site_key_breaks_the_chain() {
        let c = chain();
        let other = key(7);
        assert!(matches!(
            verify_chain(&other.public(), &c.host, &c.member, 1_000),
            Err(Error::ChainBroken)
        ));
        // A stranger signs a host certificate for our site ID under their own key.
        let forged = sign_host_certificate(&other, c.host.body.clone());
        assert!(matches!(
            verify_chain(&c.site.public(), &forged, &c.member, 1_000),
            Err(Error::ChainBroken)
        ));
        // Claiming our key as signer without our signature fails the signature check.
        let mut relabelled = forged.clone();
        relabelled.signer = c.site.public();
        assert!(matches!(
            verify_chain(&c.site.public(), &relabelled, &c.member, 1_000),
            Err(Error::BadSignature)
        ));
    }

    #[test]
    fn member_certificate_from_a_non_host_is_refused() {
        let c = chain();
        let site = c.site.public();
        // A plain member signs a certificate for a friend.
        let rogue = sign_member_certificate(&key(3), c.member.body.clone());
        assert!(matches!(
            verify_chain(&site, &c.host, &rogue, 1_000),
            Err(Error::ChainBroken)
        ));
        // The real host signs for a different site.
        let mut body = c.member.body.clone();
        body.site = key(7).site_id();
        let other_site = sign_member_certificate(&c.host_key, body);
        assert!(matches!(
            verify_chain(&site, &c.host, &other_site, 1_000),
            Err(Error::ChainBroken)
        ));
        // The real host signs under an epoch the host certificate does not carry.
        for epoch in [Epoch(3), Epoch(5)] {
            let mut body = c.member.body.clone();
            body.epoch = epoch;
            let stale = sign_member_certificate(&c.host_key, body);
            assert!(matches!(
                verify_chain(&site, &c.host, &stale, 1_000),
                Err(Error::ChainBroken)
            ));
        }
    }

    #[test]
    fn one_certificate_kind_cannot_pass_as_another() {
        // The same key signing both kinds over look-alike fields gives different signed bytes.
        let k = key(1);
        let host = HostCertificate {
            site: k.site_id(),
            host: k.device_id(),
            epoch: Epoch(1),
            deputies: vec![],
            expiry: 9,
        };
        let member = MemberCertificate {
            site: k.site_id(),
            device: k.device_id(),
            name: String::new(),
            epoch: Epoch(1),
            expiry: 9,
        };
        assert_ne!(host.signing_bytes(), member.signing_bytes());
        let as_member = k.sign(&member.signing_bytes());
        assert!(matches!(
            verify(&k.public(), &host.signing_bytes(), &as_member),
            Err(Error::BadSignature)
        ));
    }
}
