//! Pulsar wire protocol — shared by the relay server and the desktop app.
//!
//! Transport is UDP datagrams; each datagram carries exactly one
//! bincode-encoded message. Control/signaling flows client <-> relay; hole-punch
//! and media flow peer <-> peer once a direct path is established, or are
//! tunnelled through the relay as [`ClientMsg::RelayData`] / [`RelayMsg::RelayData`]
//! when P2P fails.
//!
//! The relay only ever sees opaque handshake/cipher blobs — it never holds the
//! session keys (see `pulsar-core`'s crypto module), so the relay is a
//! zero-knowledge forwarder.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Bumped on any incompatible wire change.
///
/// v2: handshake blobs now carry a 32-byte per-session salt alongside the static
/// X25519 public key (relay-path `hello`/`answer` blobs become `pubkey || salt`;
/// direct-IP `Hello`/`HelloAck` gain a `salt` field). This binds the data key to
/// the specific session and fixes cross-session key/nonce reuse — both ends must
/// run v2 code (they are rebuilt together).
///
/// v3: [`RelayMsg::PeerFound`] carries `rate_cap_kbps` — the relay's per-session
/// forwarding rate cap (0 = unlimited), so a relayed client can start its encoder at
/// or below the cap instead of blasting its full target into a throttled pipe and
/// waiting for the loss-driven ABR to converge down. bincode is positional, so the
/// added field is an incompatible wire change → both ends must run v3 (rebuilt
/// together; the relay rejects a mismatched-version Register up front).
pub const PROTOCOL_VERSION: u16 = 3;

/// Conservative MTU-safe datagram payload size.
pub const MAX_DATAGRAM: usize = 1400;

/// The relay's default UDP port (mirrors the design's `:21116`).
pub const DEFAULT_RELAY_PORT: u16 = 21116;

/// Default UDP port a host advertises for direct (relay-less) IP connects when the
/// typed target is a bare IP with no port and the peer isn't in the LAN beacon.
/// Sits next to the relay (:21116) and discovery (:21117).
pub const DEFAULT_NODE_PORT: u16 = 21118;

/// A 9-digit device identity assigned by the relay, e.g. `482913056`
/// (shown to users grouped as `482 913 056`).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceId(pub u32);

impl DeviceId {
	pub const MIN: u32 = 100_000_000;
	pub const MAX: u32 = 999_999_999;

	/// Construct from a raw number, validating the 9-digit range.
	pub fn new(n: u32) -> Option<Self> {
		if (Self::MIN..=Self::MAX).contains(&n) {
			Some(Self(n))
		} else {
			None
		}
	}

	/// A uniformly random valid id.
	pub fn random(rng: &mut impl rand::Rng) -> Self {
		Self(rng.gen_range(Self::MIN..=Self::MAX))
	}

	/// Grouped display form: `482 913 056`. Derived `Deserialize` performs no range
	/// validation, so wire data can carry an out-of-range id (e.g. `DeviceId(5)`) —
	/// fall back to the plain number instead of panicking on the 9-digit slicing
	/// (the `Debug` impl calls this on every decoded message that gets logged).
	pub fn grouped(&self) -> String {
		let s = self.0.to_string();
		if !(Self::MIN..=Self::MAX).contains(&self.0) {
			return s;
		}
		format!("{} {} {}", &s[0..3], &s[3..6], &s[6..9])
	}

	/// Parse from either grouped (`482 913 056`) or plain (`482913056`) text.
	pub fn parse(s: &str) -> Option<Self> {
		let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
		digits.parse::<u32>().ok().and_then(Self::new)
	}
}

impl std::fmt::Display for DeviceId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl std::fmt::Debug for DeviceId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "DeviceId({})", self.grouped())
	}
}

/// Opaque per-registration auth token (random 16 bytes). The client must echo it
/// back on every subsequent message so the relay can authenticate the sender.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Token(pub [u8; 16]);

impl Token {
	pub fn random(rng: &mut impl rand::RngCore) -> Self {
		let mut b = [0u8; 16];
		rng.fill_bytes(&mut b);
		Self(b)
	}

	pub fn to_hex(self) -> String {
		self.0.iter().map(|b| format!("{b:02x}")).collect()
	}

	/// Constant-time equality. Unlike the derived `PartialEq` (a byte-wise
	/// compare that short-circuits at the first mismatch), this examines every
	/// byte regardless of where they differ, so the time taken does not leak how
	/// many leading bytes matched. Use this — not `==` — for authentication
	/// checks where an attacker can observe reply timing.
	pub fn ct_eq(&self, other: &Token) -> bool {
		let mut diff = 0u8;
		for i in 0..self.0.len() {
			diff |= self.0[i] ^ other.0[i];
		}
		diff == 0
	}
}

impl std::fmt::Debug for Token {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		// Don't leak the whole token in logs.
		write!(f, "Token({}…)", &self.to_hex()[..8])
	}
}

/// A negotiated connection between two peers. Random, chosen by the requester.
pub type SessionId = u64;

/// X25519 public key (handshake material the relay forwards but never uses).
pub type PublicKey = [u8; 32];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrCode {
	TargetOffline,
	BadToken,
	NotRegistered,
	Protocol,
	RelayFull,
	/// The client's [`ClientMsg::Register`] `version` is incompatible with the
	/// relay's [`PROTOCOL_VERSION`]. Registration is refused so a version mismatch
	/// surfaces as a clear "update required" error instead of a later, misleading
	/// rendezvous/handshake timeout. Appended last so existing variant indices
	/// (bincode serializes enums by index) stay stable on the wire.
	IncompatibleVersion,
}

/// Messages sent **client → relay**.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ClientMsg {
	/// Join the relay; the relay replies with [`RelayMsg::Registered`] (assigns id + token).
	Register {
		version: u16,
		pubkey: PublicKey,
		name: Option<String>,
	},
	/// Keep the registration (and the observed public address) alive.
	Heartbeat { id: DeviceId, token: Token },
	/// Requester asks the relay to reach `target`. `hello` carries the requester's
	/// ephemeral handshake (opaque to the relay).
	Connect {
		id: DeviceId,
		token: Token,
		target: DeviceId,
		session: SessionId,
		hello: Vec<u8>,
	},
	/// Target accepts an incoming connection; `answer` carries its handshake reply.
	Accept {
		id: DeviceId,
		token: Token,
		session: SessionId,
		answer: Vec<u8>,
	},
	/// Tunnel an (already-encrypted) payload through the relay because P2P failed.
	RelayData {
		id: DeviceId,
		token: Token,
		session: SessionId,
		payload: Vec<u8>,
	},
	/// Gracefully leave.
	Bye { id: DeviceId, token: Token },
}

/// Messages sent **relay → client**.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RelayMsg {
	Registered {
		id: DeviceId,
		token: Token,
	},
	HeartbeatAck,
	/// Delivered to the **target**: someone wants to connect. Includes the
	/// requester's public address so the target can start hole-punching.
	Incoming {
		from: DeviceId,
		from_addr: SocketAddr,
		session: SessionId,
		hello: Vec<u8>,
	},
	/// Delivered to the **requester**: the target was found and accepted; includes
	/// the target's public address + handshake answer.
	PeerFound {
		target: DeviceId,
		target_addr: SocketAddr,
		session: SessionId,
		answer: Vec<u8>,
		/// The relay's PER-SESSION forwarding rate cap in **kbit/s** (`0` = unlimited /
		/// not configured). When non-zero, the requester clamps its stream's target
		/// bitrate to (a headroom-discounted) cap so a relayed session starts within the
		/// pipe instead of overshooting and waiting for the loss-driven ABR to ramp down.
		/// Only the relay-fallback path carries a meaningful value; a direct/P2P session
		/// is uncapped (`0`). v3 wire addition (see [`PROTOCOL_VERSION`]).
		rate_cap_kbps: u32,
	},
	/// A payload tunnelled from the peer (relay-fallback path).
	RelayData {
		session: SessionId,
		payload: Vec<u8>,
	},
	Error {
		code: ErrCode,
		message: String,
	},
}

/// Messages sent **peer ↔ peer** directly (after hole punching), or wrapped in
/// `RelayData` when relayed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PeerMsg {
	/// Hole-punch probe.
	Punch {
		session: SessionId,
		seq: u32,
	},
	/// Hole-punch reply — receiving one confirms the direct path works.
	PunchAck {
		session: SessionId,
		seq: u32,
	},
	/// Encrypted media/control payload.
	Data {
		session: SessionId,
		seq: u64,
		payload: Vec<u8>,
	},
	KeepAlive {
		session: SessionId,
	},
	/// Direct-IP handshake (relay-less): initiator announces its X25519 public key
	/// plus a fresh per-session salt (mixed into the data key so reconnects don't
	/// reuse a key).
	Hello {
		session: SessionId,
		pubkey: PublicKey,
		salt: [u8; 32],
	},
	/// Direct-IP handshake reply: responder returns its public key + its own fresh
	/// salt so both sides derive the same per-session key, then hole-punch as usual.
	HelloAck {
		session: SessionId,
		pubkey: PublicKey,
		salt: [u8; 32],
	},
}

/// Encode any protocol message to bytes (one datagram).
pub fn encode<T: Serialize>(msg: &T) -> Vec<u8> {
	bincode::serialize(msg).expect("protocol messages are always serializable")
}

/// Decode a protocol message from bytes.
pub fn decode<T: for<'de> Deserialize<'de>>(buf: &[u8]) -> Result<T, bincode::Error> {
	bincode::deserialize(buf)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn device_id_groups_and_parses_round_trip() {
		let id = DeviceId::new(482_913_056).unwrap();
		assert_eq!(id.grouped(), "482 913 056");
		assert_eq!(DeviceId::parse("482 913 056"), Some(id));
		assert_eq!(DeviceId::parse("482913056"), Some(id));
		assert_eq!(DeviceId::parse("ID 482-913-056!"), Some(id));
	}

	#[test]
	fn grouped_survives_out_of_range_wire_ids() {
		// Deserialize doesn't validate the range, so a crafted datagram can produce
		// e.g. DeviceId(5) — grouped()/Debug must fall back, not panic on slicing.
		assert_eq!(DeviceId(5).grouped(), "5");
		assert_eq!(DeviceId(1_000_000_000).grouped(), "1000000000");
		assert_eq!(format!("{:?}", DeviceId(5)), "DeviceId(5)");
	}

	#[test]
	fn device_id_rejects_out_of_range() {
		assert_eq!(DeviceId::new(99_999_999), None); // 8 digits
		assert_eq!(DeviceId::new(1_000_000_000), None); // 10 digits
		assert!(DeviceId::new(DeviceId::MIN).is_some());
		assert!(DeviceId::new(DeviceId::MAX).is_some());
	}

	#[test]
	fn random_device_id_is_always_nine_digits() {
		let mut rng = rand::thread_rng();
		for _ in 0..1000 {
			let id = DeviceId::random(&mut rng);
			assert_eq!(id.0.to_string().len(), 9);
			assert!(DeviceId::new(id.0).is_some());
		}
	}

	#[test]
	fn token_is_random_and_hex_is_32_chars() {
		let mut rng = rand::thread_rng();
		let a = Token::random(&mut rng);
		let b = Token::random(&mut rng);
		assert_ne!(a.0, b.0, "two random tokens collided");
		assert_eq!(a.to_hex().len(), 32);
	}

	#[test]
	fn client_messages_round_trip() {
		let mut rng = rand::thread_rng();
		let id = DeviceId::random(&mut rng);
		let token = Token::random(&mut rng);
		let msgs = vec![
			ClientMsg::Register {
				version: PROTOCOL_VERSION,
				pubkey: [7u8; 32],
				name: Some("Ev PC’si".into()),
			},
			ClientMsg::Heartbeat { id, token },
			ClientMsg::Connect {
				id,
				token,
				target: DeviceId::new(719_204_663).unwrap(),
				session: 42,
				hello: vec![1, 2, 3],
			},
			ClientMsg::RelayData {
				id,
				token,
				session: 42,
				payload: vec![9; 200],
			},
			ClientMsg::Bye { id, token },
		];
		for m in msgs {
			let bytes = encode(&m);
			let back: ClientMsg = decode(&bytes).unwrap();
			assert_eq!(m, back);
		}
	}

	#[test]
	fn relay_messages_round_trip() {
		let id = DeviceId::new(305_881_027).unwrap();
		let addr: SocketAddr = "203.0.113.5:51820".parse().unwrap();
		let msgs = vec![
			RelayMsg::Registered {
				id,
				token: Token([3u8; 16]),
			},
			RelayMsg::HeartbeatAck,
			RelayMsg::Incoming {
				from: id,
				from_addr: addr,
				session: 7,
				hello: vec![4, 5],
			},
			RelayMsg::PeerFound {
				target: id,
				target_addr: addr,
				session: 7,
				answer: vec![6, 7],
				rate_cap_kbps: 10_000,
			},
			RelayMsg::Error {
				code: ErrCode::TargetOffline,
				message: "offline".into(),
			},
		];
		for m in msgs {
			assert_eq!(decode::<RelayMsg>(&encode(&m)).unwrap(), m);
		}
	}

	/// Wire-compat regression: a relay that rejects an old-protocol client MUST send a
	/// `RelayMsg::Error` whose `ErrCode` variant is decodable by the old client.
	///
	/// Before this fix the relay sent `ErrCode::IncompatibleVersion` (variant index 5).
	/// Old clients only know indices 0..=4; bincode returns a hard error on index 5 and
	/// `handle_datagram` silently drops the datagram — the "update required" path is
	/// never reached, and the client hangs until `REGISTER_TIMEOUT`.
	///
	/// The correct reply uses `ErrCode::Protocol` (variant index 3), which every build
	/// can decode. We prove this here by encoding with a 6-variant enum (the current
	/// `ErrCode`) and decoding with a 5-variant mirror (simulating an old client).
	#[test]
	fn version_mismatch_error_decodable_by_old_client() {
		// Encode a `RelayMsg::Error { code: Protocol, message: "incompatible protocol version" }`
		// using the current (6-variant) ErrCode — this is exactly what the relay now sends.
		let wire = encode(&RelayMsg::Error {
			code: ErrCode::Protocol,
			message: "incompatible protocol version".into(),
		});

		// A 5-variant mirror of ErrCode that matches what pre-IncompatibleVersion builds
		// compiled against. Bincode decodes by variant index, so the indices 0..=4 map
		// identically — `Protocol` is index 3 in both the old 5-variant and the new
		// 6-variant enum, and must decode without error.
		#[derive(Debug, PartialEq, serde::Deserialize)]
		enum OldErrCode {
			TargetOffline,
			BadToken,
			NotRegistered,
			Protocol,
			RelayFull,
		}
		#[derive(Debug, PartialEq, serde::Deserialize)]
		enum OldRelayMsg {
			Registered {
				id: DeviceId,
				token: Token,
			},
			HeartbeatAck,
			Incoming {
				from: DeviceId,
				from_addr: std::net::SocketAddr,
				session: SessionId,
				hello: Vec<u8>,
			},
			PeerFound {
				target: DeviceId,
				target_addr: std::net::SocketAddr,
				session: SessionId,
				answer: Vec<u8>,
			},
			RelayData {
				session: SessionId,
				payload: Vec<u8>,
			},
			Error {
				code: OldErrCode,
				message: String,
			},
		}

		let decoded = decode::<OldRelayMsg>(&wire)
			.expect("old-client ErrCode (5 variants) must decode Protocol (index 3) without error");
		assert!(
			matches!(
				decoded,
				OldRelayMsg::Error {
					code: OldErrCode::Protocol,
					..
				}
			),
			"expected Protocol error, got {decoded:?}"
		);

		// Confirm the PREVIOUS (buggy) behaviour: IncompatibleVersion (index 5) is
		// undecodable by a 5-variant enum — this would have been what the old client saw.
		let buggy_wire = encode(&RelayMsg::Error {
			code: ErrCode::IncompatibleVersion,
			message: "incompatible protocol version".into(),
		});
		assert!(
			decode::<OldRelayMsg>(&buggy_wire).is_err(),
			"ErrCode index 5 must be an error for a 5-variant enum (proves the pre-fix bug)"
		);
	}

	#[test]
	fn peer_messages_round_trip() {
		let msgs = vec![
			PeerMsg::Punch { session: 1, seq: 0 },
			PeerMsg::PunchAck { session: 1, seq: 0 },
			PeerMsg::Data {
				session: 1,
				seq: 99,
				payload: vec![0xAB; 512],
			},
			PeerMsg::KeepAlive { session: 1 },
			PeerMsg::Hello {
				session: 1,
				pubkey: [9u8; 32],
				salt: [3u8; 32],
			},
			PeerMsg::HelloAck {
				session: 1,
				pubkey: [8u8; 32],
				salt: [4u8; 32],
			},
		];
		for m in msgs {
			assert_eq!(decode::<PeerMsg>(&encode(&m)).unwrap(), m);
		}
	}
}
