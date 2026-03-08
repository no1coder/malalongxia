# 麻辣龙虾 (MalaLongxia)

OpenClaw 一键安装助手，专为中国用户打造。无需命令行，无需手动配置。

[English](./README.en.md)

## 这是什么？

麻辣龙虾是一款原生桌面应用，把 OpenClaw 的整个安装流程变成简单的向导：**下一步 → 下一步 → 完成**。

它能自动帮你处理：
- **环境智能检测** — 自动检查操作系统、Node.js 版本、网络连通性
- **Node.js 自动安装** — 一键安装/升级，内置国内镜像加速（阿里云、腾讯云、清华大学）
- **OpenClaw 一键部署** — 自动完成下载安装，全程可视化进度和实时日志
- **AI API 可视化配置** — 支持多家供应商（OpenAI、Claude、国内大模型），图形界面配置 + 连接测试
- **中英双语界面** — 默认中文，一键切换英文

## 下载

| 平台 | 架构 | 文件 |
|------|------|------|
| macOS | Apple Silicon (M1/M2/M3/M4) | `OpenClawX-Full_x.x.x_aarch64.dmg` |
| macOS | Intel | `OpenClawX-Full_x.x.x_x64.dmg` |
| Windows | x64 | `OpenClawX-Full_x.x.x_x64-setup.exe` |

**Full 版本**内置 Node.js + Git，安装过程无需联网。

> macOS 用户：应用未签名。如果被 Gatekeeper 拦截，请执行：
> ```bash
> sudo xattr -cr /Applications/OpenClawX-Full.app
> ```

## 技术栈

- **桌面端**: Tauri 2 (Rust 后端)
- **前端**: React 19 + TypeScript + Vite
- **状态管理**: Zustand
- **国际化**: i18next (zh-CN, en-US)
- **图标**: lucide-react
- **样式**: CSS (暗色主题)

## 项目结构

```
src/                  # React 前端
  components/         # 可复用 UI 组件
  pages/              # 向导步骤页面
  i18n/               # 国际化
  stores/             # Zustand 状态管理
  types/              # TypeScript 类型定义
  hooks/              # 自定义 React Hooks
src-tauri/            # Rust 后端
  src/commands/       # Tauri 命令处理
```

## 开发

```bash
# 前置要求：Node.js 22+、Rust、pnpm

# 安装依赖
pnpm install

# 开发模式（Tauri 应用）
pnpm tauri dev

# 仅前端
pnpm dev

# 生产构建
pnpm tauri build

# 运行测试
pnpm test
```

## 路线图

| 版本 | 方向 | 状态 |
|------|------|------|
| v1 | 安装向导（环境检测、Node.js、OpenClaw、API 配置） | 当前版本 |
| v2 | 管理工具（配置面板、系统诊断、一键修复） | 开发中 |
| v3 | 桌面客户端（AI 对话、多模型切换、插件生态） | 规划中 |

## 许可证

MIT

## 支持

应用内扫描微信二维码，或访问 [malalongxia.com](https://malalongxia.com) 获取帮助。
