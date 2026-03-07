import { vi } from "vitest";

// Mock @tauri-apps/api/core
export const invoke = vi.fn();

// Mock @tauri-apps/api/event
export const listen = vi.fn(() => Promise.resolve(vi.fn()));

// Mock @tauri-apps/plugin-opener
export const openUrl = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen,
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl,
}));
