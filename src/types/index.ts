// Mirror source configuration for downloading Node.js / npm packages / OSS resources
export interface Mirror {
  readonly name: string;
  readonly url: string;
  readonly type: "npm" | "node" | "oss";
  readonly latency?: number;
}

// AI provider configuration
export interface AIProvider {
  readonly id: string;
  readonly name: string;
  readonly description: string;
  readonly baseUrl: string;
  readonly compatible: boolean;
}

// Single step in the installation wizard
export interface InstallStep {
  readonly id: string;
  readonly title: string;
  readonly status: "pending" | "active" | "completed" | "error" | "skipped";
}

// Result of environment detection checks
export interface EnvCheckResult {
  readonly os: "macos" | "windows" | "linux" | "unknown";
  readonly nodeVersion: string | null;
  readonly networkSpeed: "good" | "slow" | "offline";
  readonly mirrors: readonly Mirror[];
}

// Structured log entry
export interface LogEntry {
  readonly timestamp: number;
  readonly level: "info" | "warn" | "error" | "debug";
  readonly message: string;
}

// Supported language codes
export type Language = "zh-CN" | "en-US";

// Network status during checks
export type NetworkStatus = "checking" | "good" | "slow" | "offline";

// Installation lifecycle status
export type InstallStatus = "idle" | "installing" | "success" | "error";

// Operating system identifier
export type OSType = "macos" | "windows" | "linux" | "unknown";
