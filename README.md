<p align="center">
  <a href="https://pulsardesk.com"><img src=".github/assets/hero.png" alt="pulsar-proto: the language Pulsar devices speak" width="100%"></a>
</p>

<p align="center">The messages the Pulsar apps and the relay server send each other. 📨</p>

<p align="center">
  <a href="https://github.com/PulsarDesk/pulsar-proto/releases"><img src="https://img.shields.io/github/v/release/PulsarDesk/pulsar-proto?include_prereleases&style=flat-square&color=5B57E0&label=version" alt="Version"></a>
  <a href="https://github.com/PulsarDesk/pulsar-proto/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/PulsarDesk/pulsar-proto/ci.yml?branch=main&style=flat-square&label=CI" alt="CI"></a>
  <a href="https://github.com/PulsarDesk/pulsar-proto/stargazers"><img src="https://img.shields.io/github/stars/PulsarDesk/pulsar-proto?style=flat-square&color=13B3D4" alt="Stars"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPLv3-5B57E0?style=flat-square" alt="License: GPLv3"></a>
</p>

<p align="center">
  <b>Looking for the app?</b> 👉 <a href="https://github.com/PulsarDesk/pulsar-desktop"><b>Pulsar Desktop</b></a> (Windows, macOS, Linux) · <a href="https://github.com/PulsarDesk/pulsar-mobile"><b>Pulsar Mobile</b></a> (Android)
</p>

---

## 🚀 Project Status
**Current Status:** Alpha (0.x). A small Rust library, not an app. **Looking for Pulsar itself?** Get the
[desktop app](https://github.com/PulsarDesk/pulsar-desktop) or the [Android app](https://github.com/PulsarDesk/pulsar-mobile).

Both the [relay](https://github.com/PulsarDesk/relay) and [pulsar-core](https://github.com/PulsarDesk/pulsar-core)
use it, so they always agree on the message format.

---

## 📦 Other Pulsar Repositories
- [**pulsar-desktop**](https://github.com/PulsarDesk/pulsar-desktop) - Pulsar for Windows, macOS and Linux
- [**pulsar-mobile**](https://github.com/PulsarDesk/pulsar-mobile) - Pulsar for Android
- [**relay**](https://github.com/PulsarDesk/relay) - Run your own relay server
- [**pulsar-core**](https://github.com/PulsarDesk/pulsar-core) - The shared engine behind both apps

---

## 🐞 Reporting a bug
Found a problem while using Pulsar? Report it in the app's repository
([desktop](https://github.com/PulsarDesk/pulsar-desktop/issues/new/choose) or
[mobile](https://github.com/PulsarDesk/pulsar-mobile/issues/new/choose)), or in the
[relay](https://github.com/PulsarDesk/relay/issues/new/choose) if it is about your own server.

---

## 🤝 Contributing
Everyone can open [issues](https://github.com/PulsarDesk/pulsar-proto/issues) and pull requests. Irrelevant or
abusive issues may be closed or removed. Commits follow [Conventional Commits](https://www.conventionalcommits.org).

### Development Guide
```bash
git clone https://github.com/PulsarDesk/pulsar-proto.git
cd pulsar-proto
cargo test
```

```toml
[dependencies]
pulsar-proto = { git = "https://github.com/PulsarDesk/pulsar-proto" }
```

---

## 📄 License
Pulsar is licensed under **GNU GPLv3 or later** (GPL-3.0-or-later). See the [LICENSE](LICENSE) file for details.
