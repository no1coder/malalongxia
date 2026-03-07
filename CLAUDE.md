# 麻辣龙虾 (MalaLongxia)

OpenClaw 一键安装助手，专为中国用户优化。

## Tech Stack
- **Desktop**: Tauri 2 (Rust backend)
- **Frontend**: React 19 + TypeScript
- **State**: Zustand
- **i18n**: i18next + react-i18next
- **Icons**: lucide-react
- **Styling**: CSS (OpenClaw dark theme)

## Project Structure
```
src/              # React frontend
  components/     # Reusable UI components
  pages/          # Wizard step pages
  i18n/           # Internationalization
  store/          # Zustand state management
  types/          # TypeScript type definitions
  hooks/          # Custom React hooks
src-tauri/        # Rust backend
  src/commands/   # Tauri command handlers
```

## Version Roadmap
- v1: Installer wizard (env check → Node.js → OpenClaw → API config)
- v2: Management tool (config, reset, restart, diagnostics)
- v3: Desktop client (chat UI, full OpenClaw management)

## Design Guidelines
- Dark theme (#0a0a0a bg, #1a1a1a cards)
- Brand colors: Lobster Red #E54D42, Orange #F5A623
- OpenClaw official website style
- i18n: zh-CN default, en-US support

## Commands
- `pnpm tauri dev` - Development
- `pnpm tauri build` - Production build
- `pnpm dev` - Frontend only

## Conventions
- Comments in English
- Commit messages in English, conventional commits
- Immutable state patterns
- Small files (< 400 lines)
