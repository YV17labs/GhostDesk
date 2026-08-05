# Contributing to GhostDesk

Thank you for your interest in contributing to GhostDesk! This document provides guidelines and instructions for contributing to the project.

## Code of Conduct

This project adheres to a [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## Getting Started

### Prerequisites

- Docker and Docker Compose
- Git
- VS Code with the Dev Containers extension (recommended)

You do **not** need to install Rust by hand. `rust-toolchain.toml` pins the
channel, so `rustup` resolves the same compiler for everyone; the
devcontainer ships it pre-installed along with `mold` for fast links.

### Development Setup

1. **Fork the repository** on GitHub
2. **Clone your fork:**
   ```bash
   git clone https://github.com/YOUR-USERNAME/GhostDesk.git
   cd GhostDesk
   ```

3. **Create a development branch:**
   ```bash
   git checkout -b feature/your-feature-name
   ```

4. **Open the devcontainer** ("Dev Containers: Reopen in Container"), then
   start the desktop stack with the **GhostDesk: Start stack** VS Code task —
   it runs supervisord, which brings up Sway, mako, wayvnc, websockify and
   the MCP server.

5. **Build and test:**
   ```bash
   cargo build
   cargo test
   ```

6. **Run the server against the running desktop:**
   ```bash
   cargo run --bin ghostdesk
   ```
   It listens on `http://127.0.0.1:3000/mcp`. The stack's own instance is
   already on that port, so stop it first (`GhostDesk: Stop stack`) or point
   yours elsewhere with `GHOSTDESK_HTTP__PORT`.

## Making Changes

### Code Style

- `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings` both pass;
  CI enforces them
- Follow the framework's shape: a domain is an `#[injectable]` service in
  `crates/features/src/<domain>/`, wired by a `#[module]`, and the MCP
  adapter in `crates/features/src/mcp/` is the only place that knows about
  the wire
- Keep `crates/platform` free of framework types — it is the OS substrate
  (Wayland, Sway IPC, `grim`, `.desktop`) and stays testable on its own
- Comment the *why*, not the *what*; the surprising constraint is worth a
  sentence, the obvious call is not
- Write clear, descriptive commit messages

### Commits

- Use atomic commits (one logical change per commit)
- Write commit messages in the present tense ("Add feature" not "Added feature")
- Reference issues in commit messages when applicable (e.g., "Fix #123")

### Testing

- Write tests for new functionality — unit tests live in a `#[cfg(test)]`
  module beside the code they cover
- Ensure everything passes before submitting a PR:
  ```bash
  cargo fmt --all --check
  cargo clippy --all-targets --locked -- -D warnings
  cargo test --locked
  ```
- Prefer testing the decision, not the plumbing: the `.desktop` filter, the
  chord parser, the frame-diff threshold and the launch-environment scrub all
  have unit tests; the Wayland socket does not.

## Submitting Changes

### Creating a Pull Request

1. **Push your branch** to your fork:
   ```bash
   git push origin feature/your-feature-name
   ```

2. **Create a Pull Request** on GitHub with:
   - Clear title describing the change
   - Description of what was changed and why
   - Reference to any related issues (#123)
   - Test instructions if applicable

3. **PR Requirements:**
   - All CI checks must pass
   - At least one approval required
   - No merge conflicts

### PR Review Process

- A maintainer will review your PR
- Respond to feedback and make requested changes
- Keep the PR up to date with the main branch

## Reporting Issues

### Bug Reports

Include:
- Clear description of the issue
- Steps to reproduce
- Expected vs actual behavior
- Environment details (OS, image tag or `rustc --version`, etc.)
- Screenshots or error logs if applicable

### Feature Requests

Include:
- Clear description of the feature
- Use cases and motivation
- Possible implementation approach (optional)

## Project Structure

A Cargo workspace in the layout NestRS uses: one binary under `apps/`, the
domains in a shared `features` crate, and the OS substrate beside it.

```
GhostDesk/
├── apps/ghostdesk/          # The binary — wires modules, owns no logic
│   └── src/module.rs        # GhostdeskModule: HTTP + MCP + Schedule + features
├── crates/features/         # One folder per domain
│   ├── config.rs            # GhostdeskConfig — #[config(namespace = "ghostdesk")]
│   ├── screen/              # capture, stabilise, encode
│   ├── input/               # mouse, keyboard, post-action feedback
│   ├── apps/                # .desktop catalogue, launch, status
│   ├── clipboard/           # wl-copy / wl-paste
│   ├── session/             # idle watchdog (#[scheduled])
│   └── mcp/                 # the single #[mcp] host: 14 tools, 2 resources,
│                            # the bearer guard and the model-space context
├── crates/platform/         # OS substrate — no framework types
│   ├── wayland/             # virtual pointer + keyboard, XKB keymap
│   ├── sway.rs              # IPC socket discovery, tree walking
│   ├── screen.rs            # grim, frame diff, WebP/PNG
│   ├── desktop.rs           # .desktop parser
│   └── coords.rs            # model-space ↔ pixels
├── docker/                  # Base image, services, entrypoint
├── .devcontainer/           # Development container config
└── .github/workflows/       # CI/CD workflows
```

## Security

For security vulnerabilities, please refer to [SECURITY.md](SECURITY.md) for responsible disclosure guidelines.

## Questions?

Check existing issues and discussions, or open a new one.

## License

By contributing, you agree that your contributions will be licensed under the same [Functional Source License 1.1 (FSL-1.1-ALv2)](https://fsl.software/) as the project. See [LICENSE](LICENSE) for details.

Thank you for contributing to GhostDesk!
