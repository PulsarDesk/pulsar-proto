# pulsar-proto — wire protocol

Shared **relay/transport wire format** (bincode), consumed as a **git dep** by
`relay` and `pulsar-core`. After pushing here, `cargo update` in consumers.
See the umbrella `../CLAUDE.md` for the product.

- `PROTOCOL_VERSION = 3` (v3 added `PeerFound.rate_cap_kbps` — relay advertises
  a per-session rate cap so relayed clients start their encoder under it).
- `ClientMsg` (client→relay): `Register{version,pubkey,name}`, `Heartbeat`,
  `Connect{…,hello}`, `Accept{…,answer}`, `RelayData`, `Bye`.
- `RelayMsg` (relay→client): `Registered`, `HeartbeatAck`, `Incoming`,
  `PeerFound{…,rate_cap_kbps}`, `RelayData`, `Error(ErrCode)`.
- `PeerMsg` (peer↔peer, relay never sees it): `Punch`, `PunchAck`, `Data`,
  `KeepAlive`, `Hello/HelloAck{pubkey,salt}` (direct-IP, relay-less path).
- Constants: `MAX_DATAGRAM` 1400, `DEFAULT_RELAY_PORT` 21116,
  `DEFAULT_NODE_PORT` 21118. `Token` compares constant-time (`ct_eq`).

**Scope note:** the in-session app protocol (`DataMsg` side channels, service
messages) is NOT here — it lives in pulsar-core `service/wire.rs`. Only add
types here if the *relay* must understand them.

Version-bump rule: any wire-incompatible change bumps `PROTOCOL_VERSION`; the
relay rejects old versions with a wire-compatible `ErrCode::Protocol` reply.

```bash
cargo test
```
