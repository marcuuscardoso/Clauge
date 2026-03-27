# Contributing to Clauge

Thank you for your interest in contributing to Clauge! We welcome contributions from the community.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/your-username/Clauge.git`
3. Install dependencies: `bun install`
4. Run in development: `bun run tauri dev`

## Development

### Prerequisites

- [Bun](https://bun.sh) (latest)
- [Rust](https://rustup.rs) (1.77+)
- [Tauri CLI](https://tauri.app) v2

### Project Structure

- `src/` — SvelteKit frontend (Svelte 5)
- `src-tauri/` — Rust backend (Tauri v2)
- `src-tauri/src/lib.rs` — Core logic (PTY, sessions, commands)
- `src-tauri/scripts/` — Helper binaries (usage fetcher)

## Submitting Changes

1. Create a feature branch from `main`
2. Make your changes
3. Ensure `bun run build` passes
4. Ensure `cargo check` passes in `src-tauri/`
5. Submit a pull request

## Code Style

- Frontend: JavaScript with Svelte 5 runes (`$state`, `$derived`, `$effect`)
- Backend: Rust with standard formatting (`cargo fmt`)
- Use the existing patterns in the codebase

## Reporting Issues

Please use the issue templates when creating new issues.
