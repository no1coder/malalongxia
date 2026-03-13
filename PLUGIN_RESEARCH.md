# Malalongxia OpenClaw Plugin - Research & Design Document

This document contains all research findings from analyzing the OpenClaw plugin system,
to be used as context when starting the new plugin project.

## 1. Background & Goals

Malalongxia (麻辣龙虾) is an OpenClaw enhancement suite for Chinese users.
The existing `malalongxia` repo is a Tauri desktop installer (v1).
Now we need a **standalone OpenClaw plugin project** that provides:

- Configuration management (read/write OpenClaw config)
- Scheduled health checks (gateway, channels, config validation)
- LLM token/cost monitoring (real-time tracking per provider/model)
- Skill installation assistance
- Gateway status monitoring and error detection
- Auto-fix for common errors
- Chinese-optimized Web management panel
- CLI extensions
- Support for headless Linux servers (no GUI required)

## 2. Why a Plugin (Not a Standalone App)

- OpenClaw's plugin system runs **in-process** with the gateway (trusted code)
- Plugins can register HTTP routes, background services, CLI commands, hooks, gateway methods
- A plugin works on **all platforms** (macOS, Windows, Linux servers) without GUI
- Desktop users can still use the Tauri installer to set up OpenClaw + auto-install the plugin
- The official Web UI already has zh-CN translations, but they are literal translations, not localized UX

## 3. OpenClaw Plugin System - Complete API Reference

### 3.1 Plugin Entry Point

```typescript
// index.ts
import type { OpenClawPluginApi, OpenClawPluginDefinition } from "openclaw/plugin-sdk";

const plugin: OpenClawPluginDefinition = {
  id: "malalongxia",
  name: "Malalongxia",
  description: "OpenClaw China enhancement plugin",
  version: "0.1.0",
  register(api: OpenClawPluginApi) {
    // Register all components here
  },
};

export default plugin;
```

### 3.2 Plugin Manifest Files

**package.json** (key fields):
```json
{
  "name": "@malalongxia/openclaw-plugin",
  "type": "module",
  "openclaw": {
    "extensions": ["./index.ts"],
    "install": {
      "npmSpec": "@malalongxia/openclaw-plugin",
      "defaultChoice": "npm"
    }
  }
}
```

**openclaw.plugin.json**:
```json
{
  "id": "malalongxia",
  "skills": ["./skills"]
}
```

### 3.3 Full Plugin API (OpenClawPluginApi)

```typescript
interface OpenClawPluginApi {
  config: OpenClawConfig;              // Read-only config snapshot
  pluginConfig?: Record<string, unknown>; // Plugin-specific config
  runtime: PluginRuntime;              // Core runtime access

  // Registration methods:
  registerTool(factory, options?)       // Agent tools
  registerHook(name, handler)           // Lifecycle hooks (legacy)
  on(hookName, handler, options?)       // Lifecycle hooks (type-safe, preferred)
  registerHttpRoute(params)             // Web UI / HTTP endpoints
  registerCommand(definition)           // Message commands (bypass LLM)
  registerService(service)              // Background services
  registerChannel(channel)              // Messaging channels
  registerGatewayMethod(method, handler) // RPC methods
  registerCli(program, config)          // CLI command extensions
  registerProvider(provider)            // Auth providers
  registerContextEngine(engine)         // Context engines
}
```

### 3.4 Configuration Read/Write

Plugins have **full read/write access** to OpenClaw config:

```typescript
// Read
const config = api.config; // snapshot at registration time
const fresh = await api.runtime.config.loadConfig(); // reload from disk

// Write
await api.runtime.config.writeConfigFile(modifiedConfig);
```

No permission restrictions. Plugins are trusted code (run in-process).

### 3.5 Hooks System (24 hooks available)

**Agent execution hooks:**
- `before_model_resolve` - Override provider/model selection
- `before_prompt_build` - Inject system prompts/context
- `llm_input` - Observe exact LLM input payload
- `llm_output` - Observe exact LLM output payload
- `agent_end` - After conversation completes

**Message hooks:**
- `message_received` - When message arrives
- `message_sending` - Before sending (can modify/cancel)
- `message_sent` - After sent

**Tool hooks:**
- `before_tool_call` - Before tool call (can block/modify)
- `after_tool_call` - After tool call
- `tool_result_persist` - Before persisting to session

**Session hooks:**
- `session_start` / `session_end`
- `before_reset` - Before /new or /reset

**Gateway hooks:**
- `gateway_start` - Gateway started (port info available)
- `gateway_stop` - Gateway stopping (reason available)

**Subagent hooks:**
- `subagent_spawning` / `subagent_spawned` / `subagent_ended`

**Execution models:**
- Parallel (fire-and-forget): message_received/sent, agent_end, llm_input/output, gateway_start/stop
- Sequential: before_tool_call, message_sending, before_prompt_build, before_model_resolve
- Synchronous: tool_result_persist, before_message_write

### 3.6 Diagnostic Events (12 types)

Subscribe via `api.runtime.events.onDiagnosticEvent()`:

**Model usage** (`model.usage`):
```typescript
{
  type: "model.usage",
  provider?: string,   // e.g. "anthropic", "openai"
  model?: string,      // e.g. "claude-sonnet-4-5-20250514"
  usage: { input?, output?, cacheRead?, cacheWrite?, total? },
  costUsd?: number,
  durationMs?: number,
}
```

**Other diagnostic events:**
- `webhook.received` / `webhook.processed` / `webhook.error`
- `message.queued` / `message.processed`
- `session.state` - State transitions (idle/processing/waiting)
- `session.stuck` - Stuck session detection
- `queue.lane.enqueue` / `queue.lane.dequeue`
- `tool.loop` - Repetitive tool call detection
- `diagnostic.heartbeat` - Periodic summary

### 3.7 Background Services

```typescript
api.registerService({
  id: "mala-health-monitor",
  async start(ctx) {
    // Long-running service logic
    // Use setInterval for periodic checks
  },
  stop() {
    // Cleanup
  },
});
```

### 3.8 HTTP Routes (Web UI)

```typescript
api.registerHttpRoute({
  path: "/plugins/malalongxia",
  handler: async (req, res) => {
    res.writeHead(200, { "content-type": "text/html" });
    res.end("<html>...</html>");
    return true;
  },
  auth: "gateway",    // or "plugin" for custom auth
  match: "prefix",    // or "exact"
});
```

### 3.9 Message Commands

```typescript
api.registerCommand({
  name: "mala-status",     // used as /mala-status in chat
  description: "Show OpenClaw health status (Chinese)",
  acceptsArgs: false,
  requireAuth: true,
  handler: async (ctx) => {
    return { text: "Gateway is running..." };
  },
});
```

### 3.10 CLI Extensions

```typescript
api.registerCli((program, config) => {
  program
    .command("mala")
    .description("Malalongxia management commands")
    .command("status")
    .action(async () => { /* ... */ });
});
```

### 3.11 Gateway Methods (RPC)

```typescript
api.registerGatewayMethod("mala.dashboard", async (opts) => {
  const data = collectDashboardData();
  opts.respond({ ok: true, data });
});
```

### 3.12 Agent Events

```typescript
api.runtime.events.onAgentEvent((event) => {
  // event: { runId, seq, stream, ts, data, sessionKey }
  // stream: "lifecycle" | "tool" | "assistant" | "error"
});
```

### 3.13 Channel Health Monitor (built-in reference)

OpenClaw already has channel health monitoring:
- Check interval: 5 minutes
- Auto-restart unhealthy channels (max 10/hour)
- Detects: not running, not connected, busy, stuck, half-dead sockets

Our plugin can extend this with Chinese-friendly alerts and additional checks.

### 3.14 Skills (via directory, not API)

Skills are NOT registered via plugin API. Instead:
- Place `SKILL.md` files in `skills/` directory
- Declare in `openclaw.plugin.json`: `"skills": ["./skills"]`
- Tools are registered via `registerTool()` to back the skills

### 3.15 Logging

```typescript
const logger = api.runtime.logging.getChildLogger("malalongxia");
logger.info("Plugin loaded");
logger.warn("Config issue detected");
logger.error("Health check failed", { details: "..." });
```

## 4. Installation Methods

Users can install the plugin via:

```bash
# Method 1: npm (recommended, one command)
openclaw plugins install @malalongxia/openclaw-plugin

# Method 2: Local directory
openclaw plugins install ./malalongxia-plugin

# Method 3: Archive (for users without npm access)
openclaw plugins install ./malalongxia-plugin.tgz

# Method 4: Link (development)
openclaw plugins install -l ./malalongxia-plugin
```

Plugin installs to `~/.openclaw/extensions/malalongxia/` and auto-enables.

## 5. Recommended Project Structure

```
malalongxia-plugin/
├── package.json                 # name: "@malalongxia/openclaw-plugin"
├── openclaw.plugin.json         # Plugin manifest
├── index.ts                     # Plugin entry point
├── src/
│   ├── services/
│   │   ├── health-monitor.ts    # Background: periodic health checks
│   │   ├── usage-tracker.ts     # Background: token/cost tracking
│   │   └── auto-fixer.ts        # Background: common error auto-fix
│   ├── commands/
│   │   ├── status.ts            # /mala-status - overall health
│   │   ├── usage.ts             # /mala-usage  - token consumption
│   │   └── fix.ts               # /mala-fix    - one-click repair
│   ├── gateway-methods/
│   │   ├── dashboard-data.ts    # API for Web UI dashboard
│   │   └── config-api.ts        # API for config read/write
│   ├── hooks/
│   │   ├── usage-hook.ts        # Listen to model.usage events
│   │   ├── error-hook.ts        # Listen to errors, alert
│   │   └── gateway-hook.ts      # Listen to gateway start/stop
│   ├── web/
│   │   ├── dist/                # Built static assets (embedded)
│   │   ├── src/                 # Web UI source (React or Lit)
│   │   └── handler.ts           # HTTP route handler serving UI
│   ├── tools/
│   │   └── china-ai-setup.ts    # Agent tool: China AI provider setup
│   └── cli/
│       └── mala.ts              # CLI: openclaw mala [subcommand]
├── skills/
│   ├── china-setup/SKILL.md     # Skill: China environment setup guide
│   └── mirror-config/SKILL.md   # Skill: Mirror acceleration config
└── scripts/
    └── install-cn.sh            # One-line install script for China users
```

## 6. Capability Matrix

| Requirement | Supported | Mechanism |
|-------------|-----------|-----------|
| Read config | YES | `api.config` or `api.runtime.config.loadConfig()` |
| Write config | YES | `api.runtime.config.writeConfigFile()` |
| Scheduled checks | YES | `registerService()` with setInterval |
| Token monitoring | YES | `onDiagnosticEvent("model.usage")` |
| Skill management | PARTIAL | Skills via directory; tools via `registerTool()` |
| Gateway status | YES | `gateway_start/stop` hooks + health() method |
| Error detection | YES | 12 diagnostic events + session.stuck + tool.loop |
| Auto-fix errors | YES | Combine diagnostics + config write + channel restart |
| Web management UI | YES | `registerHttpRoute()` with prefix matching |
| CLI commands | YES | `registerCli()` extends `openclaw` CLI |
| Message commands | YES | `registerCommand()` for chat-based management |

## 7. Relationship with Existing Malalongxia

```
malalongxia/              # Existing repo (keep as-is)
├── src/                  # Tauri desktop installer (v1)
├── website/              # Official website
└── ...

malalongxia-plugin/       # NEW standalone repo
├── OpenClaw plugin core
├── Web management panel
└── CLI extensions

Integration:
- Tauri installer (v1) auto-runs `openclaw plugins install @malalongxia/openclaw-plugin`
- Website provides all installation method guides
- Plugin publishes to npm independently
```

## 8. Key Technical Notes

- Plugin runs **in-process** with gateway (trusted code, no sandbox)
- No fine-grained permission system; plugin has full access
- Config validation happens automatically on `writeConfigFile()`
- `register` and `activate` are synonymous in plugin lifecycle
- Use `openclaw` in `devDependencies` or `peerDependencies`, NOT in `dependencies`
- Runtime resolves `openclaw/plugin-sdk` via jiti alias
- Reference implementations: `extensions/feishu/`, `extensions/diagnostics-otel/`

## 9. OpenClaw Source Reference Paths

Key source files in the OpenClaw repo for reference:
- Plugin types: `src/plugins/types.ts`
- Plugin loader: `src/plugins/loader.ts`
- Plugin install: `src/plugins/install.ts`
- HTTP route registry: `src/plugins/http-registry.ts`
- Plugin commands: `src/plugins/commands.ts`
- Plugin runtime types: `src/plugins/runtime/types-core.ts`
- Hook runner: `src/plugins/hooks.ts`
- Diagnostic events: `src/infra/diagnostic-events.ts`
- Agent events: `src/infra/agent-events.ts`
- Channel health monitor: `src/gateway/channel-health-monitor.ts`
- Config I/O: `src/config/io.ts`
- Provider usage tracking: `src/infra/provider-usage.ts`
- Cron types: `src/cron/types.ts`
- Feishu extension (reference): `extensions/feishu/`
- Diagnostics OTel (reference): `extensions/diagnostics-otel/`
