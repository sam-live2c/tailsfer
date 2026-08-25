# Tailsfer — Installation & Usage Guide

Tailsfer is a QUIC-based LAN file-transfer system for fast device-to-device file transfers.

## Requirements

- Rust and Cargo
- Two devices connected to the same LAN
- UDP port `47691`
- A supported operating system and architecture

## Build

```bash
git clone https://github.com/sam-live2c/tailsfer.git
cd tailsfer
cargo build --release
cargo test
You're still inside the cat <<'EOF' input. The terminal is waiting for the closing EOF.

Do this now

Press:

Ctrl+C

That will cancel the incomplete file creation.

Then use a safer method that doesn't require pasting a huge heredoc:

cd ~/tailsfer-core

mkdir -p docs

cat > docs/INSTALLATION_AND_USAGE.md <<'EOF'
# Tailsfer — Installation & Usage Guide

Tailsfer is a QUIC-based LAN file-transfer system for fast device-to-device file transfers.

## Requirements

- Rust and Cargo
- Two devices connected to the same LAN
- UDP port `47691`
- A supported operating system and architecture

## Build

```bash
git clone https://github.com/sam-live2c/tailsfer.git
cd tailsfer
cargo build --release
cargo test

Receiver

Start the receiver:

./target/release/tailsfer-node

Default port:

47691/UDP

The receiver uses manual approval by default.

To automatically accept transfers:

TAILSFER_RECEIVE_POLICY=auto ./target/release/tailsfer-node

Only use automatic mode on trusted networks.

Receive Directory

Default: current directory.

Set a custom directory:

TAILSFER_RECEIVER_DIR=~/TailsferDownloads ./target/release/tailsfer-node

Send a File

./target/release/tailsfer-send <receiver-ip>:47691 <file>

Example:

./target/release/tailsfer-send 192.168.1.25:47691 test.bin

Large Files

The current high-speed pipeline uses:

1 MiB streaming buffers

QUIC transport

Streaming file I/O

BLAKE3 verification

64 MiB progress reporting

Temporary .tailsfer.part files

Large-file transfer support


Example:

./target/release/tailsfer-send \
192.168.1.25:47691 \
tailsfer-10gb-source.bin

Verification

Tailsfer calculates a BLAKE3 hash during transmission.

The receiver calculates its own hash and sends verification information back to the sender.

Android / Termux

Install:

pkg update
pkg install rust clang git

Build:

git clone https://github.com/sam-live2c/tailsfer.git
cd tailsfer
cargo build --release

Run:

./target/release/tailsfer-node

Keep Termux in the foreground during large transfers to reduce the chance of Android terminating the process.

Network

Find the receiver IP:

ip addr

Both devices should normally be on the same LAN.

Tailsfer uses:

UDP 47691

If a firewall is enabled, allow UDP traffic on this port.

Troubleshooting

Connection timeout

Check:

ip addr

Make sure:

The receiver is running.

The IP address is correct.

Both devices are on the same LAN.

UDP port 47691 is accessible.

Firewall rules are not blocking the connection.


Build failure

Try:

cargo clean
cargo build --release
cargo test

Check Rust:

rustc --version
cargo --version

Transfer interrupted

Check:

Network stability

Available storage

Available RAM

Firewall configuration

Android battery optimization

Termux background restrictions


Destination already exists

Tailsfer does not overwrite an existing destination file.

Rename or move the existing file and retry.

Testing

cargo fmt
cargo fmt --check
cargo test
git diff --check

Performance Testing

Create a 1 GiB test file:

dd if=/dev/zero of=tailsfer-1gb-source.bin bs=1M count=1024

Create a 10 GiB test file:

dd if=/dev/zero of=tailsfer-10gb-source.bin bs=1M count=10240

Run a transfer:

time ./target/release/tailsfer-send \
<receiver-ip>:47691 \
tailsfer-10gb-source.bin

Performance depends on network hardware, storage, CPU, RTT, congestion and operating-system configuration.

Current High-Speed Milestone

Release:

v0.3.0-high-speed

Commit:

0c4ed17

Security

Tailsfer uses QUIC/TLS for transport security.

The current development client uses a custom certificate verifier intended for local connectivity.

Do not expose an untrusted Tailsfer node directly to the public internet without reviewing the authentication and authorization model.

Use automatic receive mode only on trusted networks.

Known Limitations

Primarily designed for LAN transfers.

Production-grade device authentication is not yet complete.

Automatic acceptance should only be used on trusted networks.

Internet transfers may require additional networking configuration.

Performance varies between devices and networks.


Bug Reports

Include:

Device:
OS:
Architecture:
Rust version:
Cargo version:
Tailsfer version:

Sender:
Receiver:
Network:
File size:
Transfer duration:

Error:
Steps to reproduce:

Do not include passwords, private keys or other sensitive information.

License

See the repository license for the applicable terms.
