# Tailsfer

Tailsfer is a Rust-based secure remote file transfer project built around QUIC and a rendezvous/relay architecture. The goal is to enable fast, encrypted peer-to-peer or relay-assisted file transfers with a modern network stack, strong identity primitives, and clean crate boundaries.

This repository is a workspace with a shared core library and multiple service crates for rendezvous, relay, and API capabilities. It is intended as a foundation for secure file sharing over untrusted networks while maintaining strong confidentiality and integrity guarantees.

## Project vision

Tailsfer is designed around the idea that file transfer should be:

- encrypted in transit and resistant to passive interception
- resilient to NAT and network topology constraints
- usable across peer-to-peer and relay-assisted topologies
- modular enough to support future clients, APIs, and services
- implemented in a language and runtime suited to high-performance networking

The project currently uses QUIC, Tokio, and Rust cryptography primitives as the backbone for secure, low-latency communication.

## Why Tailsfer

Many existing file transfer tools are designed for convenience or local-area transfers, but they often rely on legacy protocols or trust-heavy infrastructure. Tailsfer aims to provide a modern foundation for secure transfers that can work across different network arrangements, including:

- direct device-to-device transfers
- relay-assisted transfers when direct connectivity is unavailable
- discovery through a rendezvous service
- future support for management and automation interfaces

## Architecture overview

This repository is organized as a Cargo workspace:

- `tailsfer-core` — shared logic, cryptography, transfer primitives, identity handling, and protocol building blocks
- `tailsfer-rendezvous` — registry/discovery service used to track device endpoints and help peers find one another
- `tailsfer-relay` — relay service for forwarding traffic when direct paths are blocked or unavailable
- `tailsfer-api` — API server crate for exposing management and orchestration features

### Core workspace layout

```text
.
├── Cargo.toml                # workspace configuration
├── src/                      # core library code
│   ├── crypto/               # encryption and key material management
│   ├── identity/             # device and identity logic
│   ├── rendezvous/           # rendezvous client/server concepts
│   ├── transfer/             # file transfer abstractions
│   ├── transport/            # lower-level transport components
│   └── lib.rs                # core crate entry point
├── crates/
│   ├── tailsfer-rendezvous/  # rendezvous server implementation
│   ├── tailsfer-relay/       # relay server implementation
│   └── tailsfer-api/         # API server implementation
├── testdata/                # fixtures and sample data
├── .gitignore
├── LICENSE
├── README.md
├── CONTRIBUTING.md
├── CODE_OF_CONDUCT.md
├── SECURITY.md
└── Cargo.lock
```

## Current implementation status

This repository is in active development. The workspace already includes the foundational structure for a QUIC-based transfer system and the first service implementations, including:

- a rendezvous service with device registration, heartbeat tracking, and lookup logic
- a relay service skeleton
- an API service skeleton
- cryptographic dependencies and protocol-building primitives for secure communication

The codebase is not yet a fully featured end-user application; it is best viewed as a secure transfer platform under active construction.

## Features

While the project is evolving, the design already reflects several important goals:

### Secure-by-default networking

- QUIC as the transport layer for low-latency, encrypted communication
- Rust cryptography stack with modern primitives
- explicit separation between transport, identity, and transfer logic

### Peer discovery

- device registration through a rendezvous server
- periodic heartbeat validation and expiration of stale registrations
- endpoint lookup with minimal protocol surface

### Relay support

- optional relay-assisted connectivity for networks behind NAT or firewalls
- architecture designed to support traffic forwarding between peers

### Extensible service model

- isolated crates for different responsibilities
- workspace layout that allows independent service evolution
- simple API layer for future control-plane integrations

## Technology stack

The repository uses:

- Rust
- Tokio
- QUIC via Quinn
- Rustls
- Tokio runtime and async I/O
- cryptographic building blocks such as BLAKE3, ChaCha20-Poly1305, SHA-2, and zeroization-based secure handling

## Prerequisites

To build and work on this project, install the Rust toolchain:

```bash
curl https://sh.rustup.rs -sSf | sh
source "$HOME/.cargo/env"
```

Then verify your installation:

```bash
rustc --version
cargo --version
```

## Building the project

Clone the repository:

```bash
git clone https://github.com/sam-live29/tailsfer.git
cd tailsfer
```

Build the workspace:

```bash
cargo build
```

Run the full workspace tests if available:

```bash
cargo test
```

## Running the services

The project is organized as a workspace, so individual services can be started independently.

### Rendezvous service

```bash
cargo run -p tailsfer-rendezvous
```

This starts the service listening on `0.0.0.0:47690` with a simple line-based registration protocol. It supports commands such as:

- `REGISTER <device_id> <ip:port>`
- `HEARTBEAT <device_id>`
- `LOOKUP <device_id>`
- `PING`

### Relay service

```bash
cargo run -p tailsfer-relay
```

### API service

```bash
cargo run -p tailsfer-api
```

## Expected runtime behavior

The rendezvous server currently demonstrates the core discovery model:

- devices register their endpoint
- each registration has a freshness timestamp
- stale records are purged automatically
- clients can query endpoint information by device ID

This makes the project well suited for a future direct-transfer and relay orchestration layer.

## Security model

Tailsfer is designed around secure transfer principles. In broad terms, the codebase aims to support:

- authenticated endpoints
- encrypted transfer payloads
- protection against tampering and replay-style misuse
- controlled trust boundaries between discovery, relay, and application logic

The project is still under development, so security review and hardening should be considered part of the active roadmap.

## Roadmap and future work

The repository suggests several natural next steps:

- harden and complete the transport layer
- expand the file-transfer protocol and session lifecycle
- add better identity and authentication flows
- improve relay load balancing and failover logic
- build a client CLI or GUI experience
- define an external API contract for integration with other services
- add end-to-end tests and fuzz coverage

## Contributing

Contributions are welcome. Please read [`CONTRIBUTING.md`](CONTRIBUTING.md) for development guidelines, coding expectations, and the pull request process.

## Code of conduct

This project follows a standard community code of conduct. Please review [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) before participating.

## Security

If you discover a security issue, please report it responsibly. See [`SECURITY.md`](SECURITY.md) for the disclosure process.

## License

This project is licensed under the Apache License, Version 2.0. See [`LICENSE`](LICENSE) for the full text.

## Maintainers and project status

Tailsfer is a community-driven project with the goal of enabling secure, efficient, and practical file transfer over modern networking protocols. This repository is designed to evolve over time as the protocol, security model, and service architecture mature.

## A note on maturity

Because this repository is still in development, interfaces, commands, and package boundaries may change frequently. Contributors are encouraged to keep changes focused, well-documented, and aligned with the project’s security and modularity goals.

---

Thank you for your interest in Tailsfer. If you are exploring the codebase, contributing improvements, or building on the project, we welcome your feedback and collaboration.
