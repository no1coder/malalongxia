# MalaLongxia (麻辣龙虾)

OpenClaw one-click installer for Chinese users. No terminal, no configuration, no frustration.

[中文](./README.md)

## What is this?

MalaLongxia is a native desktop app that turns the entire OpenClaw setup process into a simple wizard: **Next → Next → Done**.

It handles everything automatically:
- **Environment detection** — checks OS, Node.js version, network connectivity
- **Node.js installation** — auto-install/upgrade with domestic mirror acceleration (Aliyun, Tencent, Tsinghua)
- **OpenClaw deployment** — one-click install with real-time progress and logs
- **AI API configuration** — visual setup for multiple providers (OpenAI, Claude, domestic LLMs), with connection testing
- **Bilingual UI** — Chinese (default) and English

## Download

| Platform | Architecture | File |
|----------|-------------|------|
| macOS | Apple Silicon (M1/M2/M3/M4) | `OpenClawX-Full_x.x.x_aarch64.dmg` |
| macOS | Intel | `OpenClawX-Full_x.x.x_x64.dmg` |
| Windows | x64 | `OpenClawX-Full_x.x.x_x64-setup.exe` |

**Full edition** bundles Node.js + Git — no internet required during setup.

> macOS users: the app is unsigned. If blocked by Gatekeeper, run:
> ```bash
> sudo xattr -cr /Applications/OpenClawX-Full.app
> ```

## Tech Stack

- **Desktop**: Tauri 2 (Rust backend)
- **Frontend**: React 19 + TypeScript + Vite
- **State**: Zustand
- **i18n**: i18next (zh-CN, en-US)
- **Icons**: lucide-react
- **Styling**: CSS (dark theme)

## Project Structure

```
src/                  # React frontend
  components/         # Reusable UI components
  pages/              # Wizard step pages
  i18n/               # Internationalization
  stores/             # Zustand state management
  types/              # TypeScript type definitions
  hooks/              # Custom React hooks
src-tauri/            # Rust backend
  src/commands/       # Tauri command handlers
```

## Development

```bash
# Prerequisites: Node.js 22+, Rust, pnpm

# Install dependencies
pnpm install

# Development (Tauri app)
pnpm tauri dev

# Frontend only
pnpm dev

# Production build
pnpm tauri build

# Run tests
pnpm test
```

## Roadmap

| Version | Focus | Status |
|---------|-------|--------|
| v1 | Installer wizard (env check, Node.js, OpenClaw, API config) | Current |
| v2 | Management tool (config panel, diagnostics, repair) | In progress |
| v3 | Desktop client (chat UI, multi-model, plugin ecosystem) | Planned |

## License

MIT

## Support

Scan the WeChat QR code in the app or visit [malalongxia.com](https://malalongxia.com) for help.
