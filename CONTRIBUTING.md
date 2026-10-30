# Contributing to Tailsfer

Thanks for your interest in contributing to Tailsfer.

We welcome improvements, bug reports, security disclosures, documentation updates, and feature ideas. This project is built in the open and benefits from careful review and clear communication.

## Ways to contribute

- report bugs or unexpected behavior
- suggest new features or protocol improvements
- improve documentation and examples
- propose performance or security improvements
- submit fixes and refactors with accompanying rationale

## Development workflow

1. Fork the repository and create a feature branch.
2. Keep your changes focused and easy to review.
3. Build the project locally before opening a pull request.
4. Run formatting and validation tools when applicable.
5. Open a clear pull request with a summary, testing notes, and rationale.

## Local setup

```bash
git clone https://github.com/sam-live29/tailsfer.git
cd tailsfer
cargo build
cargo test
```

## Coding expectations

- prefer simple, readable Rust code
- keep changes aligned with the modular workspace architecture
- document important protocol or security-relevant changes
- avoid unrelated formatting churn
- ensure new code is maintainable and testable

## Formatting and validation

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

If a project area is still experimental, document the status clearly in the PR description.

## Pull requests

Please open a pull request that includes:

- a concise summary of the change
- the reason for the change
- any testing performed
- any known limitations or follow-up work

Large changes should be discussed first through an issue or a design discussion when possible.

## Reporting issues

Please search existing issues before opening a new one. Include:

- reproducible steps
- expected behavior
- actual behavior
- environment details (OS, Rust version, network conditions, relevant logs)

## Conduct

Please follow the project code of conduct in [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md).

## Licensing

By contributing to this project, you agree that your contributions will be licensed under the Apache License 2.0.
