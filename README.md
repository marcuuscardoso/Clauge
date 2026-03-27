<p align="center">
  <img src="src-tauri/icons/icon.png" alt="Clauge" width="120" />
</p>

<h1 align="center">Clauge</h1>

<p align="center">
  Session manager for Claude Code
</p>

<p align="center">
  <a href="https://github.com/ansxuman/Clauge/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-Apache%202.0-7c5cf8?style=flat-square" alt="License"></a>
  <a href="https://github.com/ansxuman/Clauge/stargazers"><img src="https://img.shields.io/github/stars/ansxuman/Clauge?style=flat-square&color=f5a623" alt="Stars"></a>
  <a href="https://github.com/ansxuman/Clauge/issues"><img src="https://img.shields.io/github/issues/ansxuman/Clauge?style=flat-square&color=4f94d4" alt="Issues"></a>
  <a href="https://github.com/ansxuman/Clauge/releases/latest"><img src="https://img.shields.io/github/v/release/ansxuman/Clauge?style=flat-square&color=1dc880" alt="Release"></a>
</p>

<p align="center">
  <a href="https://github.com/ansxuman/Clauge/issues">Report Bug</a> ·
  <a href="https://github.com/ansxuman/Clauge/issues">Request Feature</a> ·
  <a href="https://buymeacoffee.com/ansxuman">Buy me a coffee</a>
</p>

---

## Features

- **Session profiles** with title and purpose — Brainstorming, Development, Code Review, Debugging
- **Embedded terminal** (xterm.js + PTY) — no external terminal window needed
- **Multi-session** — switch between active sessions without re-spawning
- **Project grouping** with expand/collapse
- **Auto-discovery** of existing Claude Code sessions from `~/.claude/projects/`
- **Usage tracking** — session and weekly limits shown in the menu bar
- **macOS native** — vibrancy, hidden titlebar, system tray
- **Dark & light themes** with accent color picker
- **Keyboard shortcuts** — `Cmd+N`, `Cmd+1-9`
- **Context prompts** — auto-injected based on session purpose

## Download

<a href="https://github.com/ansxuman/Clauge/releases/latest"><strong>Download for macOS →</strong></a>

## Development

**Requires:** [Bun](https://bun.sh), [Rust](https://rustup.rs) 1.77+, [Tauri CLI](https://tauri.app) v2

```bash
git clone https://github.com/ansxuman/Clauge.git
cd Clauge
bun install
bun run tauri dev
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Frontend | SvelteKit + Svelte 5 |
| Backend | Rust + Tauri v2 |
| Terminal | xterm.js + portable-pty |
| Usage API | Swift (NSURLSession) |

## Contributing

See [CONTRIBUTING.md](.github/CONTRIBUTING.md).

## License

[Apache License 2.0](LICENSE)
