//! Relay `proto` module (error prefix `RLY-PR`). Wire message types, encoding, version numbers. No I/O.
//!
//! Boundaries and rules: see `docs/design/01-architecture.md` and
//! `docs/agent/modules/proto.md`.
//!
//! # Frame layout
//!
//! All integers are big-endian.
//!
//! ```text
//! +----------+-----------+-----------+------------------+
//! | len: u32 | version:  | type: u16 | payload          |
//! |          | u16       |           | (len - 4 bytes)  |
//! +----------+-----------+-----------+------------------+
//! ```
//!
//! `len` counts every byte after the length field itself: 2 (version) + 2 (type) + payload.
//! So `len >= 4`, and `len <= MAX_FRAME_LEN`. Frames of unknown type decode to
//! [`FrameType::Unknown`] so callers can skip them.

use relay_common::{ErrorCode, Module, RelayError};

/// This crate's module tag, used in error codes and health reports.
pub const MODULE: Module = Module::Proto;

/// The wire version written by this build.
pub const PROTOCOL_VERSION: u16 = 1;

/// Largest allowed value of the length field (version + type + payload), in bytes.
pub const MAX_FRAME_LEN: usize = 4 * 1024 * 1024;

/// Bytes taken by the length prefix.
const LEN_BYTES: usize = 4;
/// Bytes taken by version and type.
const HEADER_BYTES: usize = 4;

/// Errors raised by the `proto` module.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The feature exists in the design but is not built yet.
    #[error("{code} not implemented yet: {what}", code = ErrorCode::new(MODULE, 1))]
    NotImplemented {
        /// The missing feature.
        what: &'static str,
    },
    /// The frame is larger than [`MAX_FRAME_LEN`].
    #[error("{code} frame of {len} bytes exceeds the limit of {MAX_FRAME_LEN}", code = ErrorCode::new(MODULE, 2))]
    TooLarge {
        /// The offending length.
        len: usize,
    },
    /// The buffer ends before the frame does; read more bytes and try again.
    #[error("{code} incomplete frame, need {needed} more bytes", code = ErrorCode::new(MODULE, 3))]
    Truncated {
        /// How many more bytes are needed at least.
        needed: usize,
    },
    /// The length field is too small to hold version and type.
    #[error("{code} length field {len} is too small", code = ErrorCode::new(MODULE, 4))]
    BadLength {
        /// The offending length.
        len: usize,
    },
}

impl RelayError for Error {
    fn code(&self) -> ErrorCode {
        match self {
            Error::NotImplemented { .. } => ErrorCode::new(MODULE, 1),
            Error::TooLarge { .. } => ErrorCode::new(MODULE, 2),
            Error::Truncated { .. } => ErrorCode::new(MODULE, 3),
            Error::BadLength { .. } => ErrorCode::new(MODULE, 4),
        }
    }
}

/// Result type for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// What a frame carries.
///
/// Reserved for later phases (not defined yet): P1 transfer offer / accept / chunk / complete;
/// P2 join request / approve / roster / revoke and folder list / read; P3 host change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameType {
    /// First message from the opener of a session (type 1).
    Hello,
    /// Reply to [`FrameType::Hello`] (type 2).
    HelloAck,
    /// Liveness / round-trip probe (type 3).
    Ping,
    /// Reply to [`FrameType::Ping`] (type 4).
    Pong,
    /// A type this build does not know. Skip the frame.
    Unknown(u16),
}

impl FrameType {
    /// The number written on the wire.
    pub const fn to_u16(self) -> u16 {
        match self {
            FrameType::Hello => 1,
            FrameType::HelloAck => 2,
            FrameType::Ping => 3,
            FrameType::Pong => 4,
            FrameType::Unknown(n) => n,
        }
    }

    /// Maps a wire number to a type; unrecognised numbers become [`FrameType::Unknown`].
    pub const fn from_u16(n: u16) -> Self {
        match n {
            1 => FrameType::Hello,
            2 => FrameType::HelloAck,
            3 => FrameType::Ping,
            4 => FrameType::Pong,
            n => FrameType::Unknown(n),
        }
    }
}

/// One message on the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Protocol version the sender used.
    pub version: u16,
    /// What the payload is.
    pub frame_type: FrameType,
    /// Type-specific bytes.
    pub payload: Vec<u8>,
}

impl Frame {
    /// A frame at [`PROTOCOL_VERSION`].
    pub fn new(frame_type: FrameType, payload: Vec<u8>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            frame_type,
            payload,
        }
    }

    /// Serialises the frame. Fails with [`Error::TooLarge`] past [`MAX_FRAME_LEN`].
    pub fn encode(&self) -> Result<Vec<u8>> {
        let len = HEADER_BYTES + self.payload.len();
        if len > MAX_FRAME_LEN {
            return Err(Error::TooLarge { len });
        }
        let mut out = Vec::with_capacity(LEN_BYTES + len);
        out.extend_from_slice(&(len as u32).to_be_bytes());
        out.extend_from_slice(&self.version.to_be_bytes());
        out.extend_from_slice(&self.frame_type.to_u16().to_be_bytes());
        out.extend_from_slice(&self.payload);
        Ok(out)
    }

    /// Reads one frame from the start of `buf`.
    ///
    /// Returns the frame and how many bytes it used, so a caller can keep decoding the rest.
    /// [`Error::Truncated`] means "feed me more bytes", not a corrupt stream. The length limit
    /// is checked before the payload is waited for, so a huge claimed length fails at once.
    pub fn decode(buf: &[u8]) -> Result<(Frame, usize)> {
        if buf.len() < LEN_BYTES {
            return Err(Error::Truncated {
                needed: LEN_BYTES - buf.len(),
            });
        }
        let len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
        if len > MAX_FRAME_LEN {
            return Err(Error::TooLarge { len });
        }
        if len < HEADER_BYTES {
            return Err(Error::BadLength { len });
        }
        let total = LEN_BYTES + len;
        if buf.len() < total {
            return Err(Error::Truncated {
                needed: total - buf.len(),
            });
        }
        let version = u16::from_be_bytes([buf[4], buf[5]]);
        let frame_type = FrameType::from_u16(u16::from_be_bytes([buf[6], buf[7]]));
        let payload = buf[LEN_BYTES + HEADER_BYTES..total].to_vec();
        Ok((
            Frame {
                version,
                frame_type,
                payload,
            },
            total,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_error_code_has_module_prefix() {
        let all = [
            (Error::NotImplemented { what: "x" }, "RLY-PR-001"),
            (Error::TooLarge { len: 1 }, "RLY-PR-002"),
            (Error::Truncated { needed: 1 }, "RLY-PR-003"),
            (Error::BadLength { len: 1 }, "RLY-PR-004"),
        ];
        for (e, code) in all {
            assert_eq!(e.code().to_string(), code);
            assert!(e.to_string().starts_with(code), "{e}");
        }
    }

    #[test]
    fn round_trip_all_known_types() {
        for t in [
            FrameType::Hello,
            FrameType::HelloAck,
            FrameType::Ping,
            FrameType::Pong,
        ] {
            let f = Frame::new(t, vec![1, 2, 3]);
            let bytes = f.encode().unwrap();
            let (back, used) = Frame::decode(&bytes).unwrap();
            assert_eq!(back, f);
            assert_eq!(used, bytes.len());
        }
    }

    #[test]
    fn layout_is_as_documented() {
        let f = Frame {
            version: 0x0102,
            frame_type: FrameType::Ping,
            payload: vec![9],
        };
        assert_eq!(f.encode().unwrap(), vec![0, 0, 0, 5, 0x01, 0x02, 0, 3, 9]);
    }

    #[test]
    fn empty_payload_and_trailing_bytes() {
        let mut bytes = Frame::new(FrameType::Pong, vec![]).encode().unwrap();
        let used_expected = bytes.len();
        bytes.extend_from_slice(&[0xEE, 0xEE]);
        let (f, used) = Frame::decode(&bytes).unwrap();
        assert!(f.payload.is_empty());
        assert_eq!(used, used_expected);
    }

    #[test]
    fn two_frames_back_to_back() {
        let a = Frame::new(FrameType::Ping, vec![1]);
        let b = Frame::new(FrameType::Pong, vec![2, 2]);
        let mut bytes = a.encode().unwrap();
        bytes.extend(b.encode().unwrap());
        let (fa, n) = Frame::decode(&bytes).unwrap();
        let (fb, _) = Frame::decode(&bytes[n..]).unwrap();
        assert_eq!((fa, fb), (a, b));
    }

    #[test]
    fn truncated_input_reports_how_much_is_missing() {
        let bytes = Frame::new(FrameType::Hello, vec![0; 10]).encode().unwrap();
        for cut in 0..bytes.len() {
            match Frame::decode(&bytes[..cut]) {
                Err(Error::Truncated { needed }) => {
                    assert!(needed >= 1);
                    assert!(cut + needed <= bytes.len());
                }
                other => panic!("cut {cut}: {other:?}"),
            }
        }
    }

    #[test]
    fn oversize_is_rejected_both_ways() {
        let f = Frame::new(FrameType::Hello, vec![0; MAX_FRAME_LEN]);
        assert!(matches!(f.encode(), Err(Error::TooLarge { .. })));
        // Claimed length over the limit fails before any payload arrives.
        let claimed = (MAX_FRAME_LEN as u32 + 1).to_be_bytes();
        assert!(matches!(
            Frame::decode(&claimed),
            Err(Error::TooLarge { .. })
        ));
        // Exactly at the limit is fine.
        let ok = Frame::new(FrameType::Hello, vec![0; MAX_FRAME_LEN - 4]);
        assert!(ok.encode().is_ok());
    }

    #[test]
    fn length_below_header_is_bad() {
        let bytes = [0, 0, 0, 3, 0, 1, 0];
        assert!(matches!(
            Frame::decode(&bytes),
            Err(Error::BadLength { len: 3 })
        ));
    }

    #[test]
    fn unknown_type_decodes_and_round_trips() {
        let f = Frame::new(FrameType::Unknown(900), vec![5, 6]);
        let bytes = f.encode().unwrap();
        let (back, used) = Frame::decode(&bytes).unwrap();
        assert_eq!(back.frame_type, FrameType::Unknown(900));
        assert_eq!(back.payload, vec![5, 6]);
        assert_eq!(used, bytes.len());
        assert_eq!(FrameType::from_u16(1), FrameType::Hello);
        assert_eq!(FrameType::from_u16(0), FrameType::Unknown(0));
    }

    #[test]
    fn newer_version_still_decodes() {
        let f = Frame {
            version: PROTOCOL_VERSION + 5,
            frame_type: FrameType::Ping,
            payload: vec![],
        };
        let (back, _) = Frame::decode(&f.encode().unwrap()).unwrap();
        assert_eq!(back.version, PROTOCOL_VERSION + 5);
    }
}
