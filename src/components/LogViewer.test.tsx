import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import type { LogEntry } from "../types";

// Mock i18n before importing component
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: "zh-CN", changeLanguage: vi.fn() },
  }),
}));

import { LogViewer } from "./LogViewer";

describe("LogViewer", () => {
  it("renders empty state when no logs", () => {
    render(<LogViewer logs={[]} />);
    expect(screen.getByText("log.empty")).toBeInTheDocument();
  });

  it("renders log entries with messages", () => {
    const logs: LogEntry[] = [
      { timestamp: 1700000000000, level: "info", message: "Starting install" },
      { timestamp: 1700000001000, level: "error", message: "Install failed" },
    ];
    render(<LogViewer logs={logs} />);
    expect(screen.getByText("Starting install")).toBeInTheDocument();
    expect(screen.getByText("Install failed")).toBeInTheDocument();
  });

  it("renders log level badges", () => {
    const logs: LogEntry[] = [
      { timestamp: 1700000000000, level: "info", message: "test" },
      { timestamp: 1700000001000, level: "warn", message: "warning" },
    ];
    render(<LogViewer logs={logs} />);
    expect(screen.getByText("[INFO]")).toBeInTheDocument();
    expect(screen.getByText("[WARN]")).toBeInTheDocument();
  });

  it("applies level-specific CSS classes", () => {
    const logs: LogEntry[] = [
      { timestamp: 1700000000000, level: "error", message: "err msg" },
    ];
    const { container } = render(<LogViewer logs={logs} />);
    const line = container.querySelector(".log-viewer__line--error");
    expect(line).toBeInTheDocument();
  });

  it("applies maxHeight style", () => {
    const { container } = render(<LogViewer logs={[]} maxHeight="200px" />);
    const viewer = container.querySelector(".log-viewer");
    expect(viewer).toHaveStyle({ maxHeight: "200px" });
  });

  it("renders timestamps in HH:MM:SS format", () => {
    // 2023-11-14 22:13:20 UTC = specific time
    const logs: LogEntry[] = [
      { timestamp: 1700000000000, level: "info", message: "test" },
    ];
    const { container } = render(<LogViewer logs={logs} />);
    const timestamp = container.querySelector(".log-viewer__timestamp");
    expect(timestamp?.textContent).toMatch(/\d{2}:\d{2}:\d{2}/);
  });
});
