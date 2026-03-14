import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const { mockGoToStep, mockInvoke, mockListen } = vi.hoisted(() => ({
  mockGoToStep: vi.fn(),
  mockInvoke: vi.fn(),
  mockListen: vi.fn(() => Promise.resolve(vi.fn())),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown>) => {
      if (opts) return `${key} ${JSON.stringify(opts)}`;
      return key;
    },
    i18n: { language: "zh-CN", changeLanguage: vi.fn() },
  }),
}));

vi.mock("../hooks/useStepNavigation", () => ({
  useStepNavigation: () => ({
    goToStep: mockGoToStep,
    goNext: vi.fn(),
    goPrev: vi.fn(),
    STEP_ROUTES: ["/", "/env-check", "/node-install", "/openclaw-install", "/api-config", "/completion"],
  }),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mockInvoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: mockListen }));

vi.mock("../hooks/useMirrorConfig", () => ({
  useMirrorConfig: () => ({
    nodeMirrors: [],
    npmMirrors: [
      { name: "mirror.npmmirror", url: "https://registry.npmmirror.com", type: "npm" },
      { name: "mirror.tencent", url: "https://mirrors.cloud.tencent.com/npm/", type: "npm" },
      { name: "mirror.huawei", url: "https://repo.huaweicloud.com/repository/npm/", type: "npm" },
      { name: "mirror.official", url: "https://registry.npmjs.org", type: "npm" },
    ],
    nvmInstallScript: null,
    nodeVersion: null,
    isLoading: false,
    isRemote: false,
  }),
}));

import { useInstallStore } from "../stores/useInstallStore";
import OpenClawInstallPage from "./OpenClawInstallPage";
import { renderWithRouter } from "../test/render";

describe("OpenClawInstallPage", () => {
  beforeEach(() => {
    mockGoToStep.mockClear();
    mockInvoke.mockClear();
    mockListen.mockClear();
    mockListen.mockImplementation(() => Promise.resolve(vi.fn()));
    // Mock invoke calls: verify_node_npm returns npm_available, test_mirror_latency returns ms
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "verify_node_npm") return Promise.resolve({ npm_available: true, node_version: "v22.22.0", npm_version: "10.9.2" });
      if (cmd === "test_mirror_latency") return Promise.resolve(100);
      if (cmd === "install_openclaw") return Promise.resolve({ version: "1.0.0" });
      return Promise.resolve(null);
    });
    useInstallStore.setState({
      openclawInstallStatus: "idle",
      openclawInstallLogs: [],
      openclawVersion: null,
    });
  });

  it("renders title and description", () => {
    renderWithRouter(<OpenClawInstallPage />);
    expect(screen.getByText("openclawInstall.title")).toBeInTheDocument();
    expect(screen.getByText("openclawInstall.description")).toBeInTheDocument();
  });

  it("renders strategy info banner", () => {
    renderWithRouter(<OpenClawInstallPage />);
    expect(screen.getByText("openclawInstall.strategy")).toBeInTheDocument();
  });

  it("renders mirror select header", () => {
    renderWithRouter(<OpenClawInstallPage />);
    expect(screen.getByText("openclawInstall.selectMirror")).toBeInTheDocument();
  });

  it("renders all NPM mirror sources", () => {
    renderWithRouter(<OpenClawInstallPage />);
    expect(screen.getByText("mirror.npmmirror")).toBeInTheDocument();
    expect(screen.getByText("mirror.tencent")).toBeInTheDocument();
    expect(screen.getByText("mirror.huawei")).toBeInTheDocument();
    expect(screen.getByText("mirror.official")).toBeInTheDocument();
  });

  it("shows mirror URLs", () => {
    renderWithRouter(<OpenClawInstallPage />);
    expect(screen.getByText("https://registry.npmmirror.com")).toBeInTheDocument();
    expect(screen.getByText("https://registry.npmjs.org")).toBeInTheDocument();
  });

  it("highlights selected mirror", () => {
    const { container } = renderWithRouter(<OpenClawInstallPage />);
    const selected = container.querySelector(".ocinstall-mirror.selected");
    expect(selected).toBeInTheDocument();
  });

  it("navigates back to step 1 when node was not required", async () => {
    const user = userEvent.setup();
    renderWithRouter(<OpenClawInstallPage />);

    await user.click(screen.getByText("btn.prev"));
    // When nodeRequired=false (default), skip NodeInstallPage and go to EnvCheck
    expect(mockGoToStep).toHaveBeenCalledWith(1);
  });

  it("starts install on install button click when idle", async () => {
    const user = userEvent.setup();
    renderWithRouter(<OpenClawInstallPage />);

    // Wait for npm check and mirror auto-selection to complete
    await waitFor(() => {
      const btn = screen.getByText("openclawInstall.installBtn");
      expect(btn).not.toBeDisabled();
    });

    await user.click(screen.getByText("openclawInstall.installBtn"));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("install_openclaw", {
        mirror: expect.stringContaining("registry"),
      });
    });
  });

  it("disables buttons during install", () => {
    useInstallStore.setState({ openclawInstallStatus: "installing" });
    renderWithRouter(<OpenClawInstallPage />);

    // "openclawInstall.installing" appears in both progress status and button
    const installingElements = screen.getAllByText("openclawInstall.installing");
    // The button should be disabled
    const nextBtn = installingElements.find((el) => el.tagName === "BUTTON");
    expect(nextBtn).toBeDisabled();

    const backBtn = screen.getByText("btn.prev");
    expect(backBtn).toBeDisabled();
  });

  it("registers event listeners on mount", () => {
    renderWithRouter(<OpenClawInstallPage />);
    expect(mockListen).toHaveBeenCalledWith("openclaw-install-progress", expect.any(Function));
    expect(mockListen).toHaveBeenCalledWith("openclaw-install-log", expect.any(Function));
  });

  it("calls test_mirror_latency on mount for speed test", async () => {
    renderWithRouter(<OpenClawInstallPage />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("test_mirror_latency", expect.objectContaining({
        url: expect.any(String),
      }));
    });
  });

  it("speed test button is present", async () => {
    renderWithRouter(<OpenClawInstallPage />);
    // Button shows "mirror.testing" during auto-test, then "mirror.testSpeed" after
    await waitFor(() => {
      const btn = screen.getByText(/mirror\.(testSpeed|testing)/);
      expect(btn).toBeInTheDocument();
    });
  });

  it("shows install logs when present", () => {
    useInstallStore.setState({
      openclawInstallLogs: [
        { timestamp: 1000, level: "info", message: "Setting npm registry..." },
      ],
    });
    renderWithRouter(<OpenClawInstallPage />);
    expect(screen.getByText("Setting npm registry...")).toBeInTheDocument();
  });

  it("allows retry after error", async () => {
    const user = userEvent.setup();
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === "verify_node_npm") return Promise.resolve({ npm_available: true, node_version: "v22.22.0", npm_version: "10.9.2" });
      if (cmd === "test_mirror_latency") return Promise.resolve(100);
      if (cmd === "install_openclaw") return Promise.resolve({ version: "1.0.0" });
      return Promise.resolve(null);
    });
    useInstallStore.setState({ openclawInstallStatus: "error" });
    renderWithRouter(<OpenClawInstallPage />);

    // Wait for npm check and mirror selection to complete, then retry button appears
    await waitFor(() => {
      const btn = screen.getByText("btn.retry");
      expect(btn).not.toBeDisabled();
    });

    await user.click(screen.getByText("btn.retry"));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("install_openclaw", expect.any(Object));
    });
  });

  it("navigates to step 4 on next click when success", async () => {
    const user = userEvent.setup();
    useInstallStore.setState({ openclawInstallStatus: "success" });
    renderWithRouter(<OpenClawInstallPage />);

    // In success state, btn.next should be enabled
    const nextBtn = screen.getByText("btn.next");
    expect(nextBtn).not.toBeDisabled();

    await user.click(nextBtn);
    expect(mockGoToStep).toHaveBeenCalledWith(4);
  });

  it("selects a different mirror on click", async () => {
    const user = userEvent.setup();
    renderWithRouter(<OpenClawInstallPage />);

    await user.click(screen.getByText("mirror.official"));
    // The clicked mirror should now have the "selected" class
    const officialMirror = screen.getByText("mirror.official").closest(".ocinstall-mirror");
    expect(officialMirror?.classList.contains("selected")).toBe(true);
  });
});
