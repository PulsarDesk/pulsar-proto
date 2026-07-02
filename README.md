# pulsar-proto

Shared **wire protocol** for [Pulsar](https://github.com/PulsarDesk) — the
bincode-encoded messages exchanged between the relay/rendezvous server and the
app (client + host).

It is the single source of truth for the relay ↔ app contract, so both sides
stay compile-time compatible. Consumed as a git dependency by:

- [`PulsarDesk/relay`](https://github.com/PulsarDesk/relay) — the rendezvous / relay server
- [`PulsarDesk/pulsar`](https://github.com/PulsarDesk/pulsar) — the desktop + mobile app

```toml
[dependencies]
pulsar-proto = { git = "https://github.com/PulsarDesk/pulsar-proto" }
```

## Build & test

```bash
cargo test
```

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
