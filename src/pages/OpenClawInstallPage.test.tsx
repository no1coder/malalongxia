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

import { useInstallStore } from "../stores/useInstallStore";
import OpenClawInstallPage from "./OpenClawInstallPage";
import { renderWithRouter } from "../test/render";

describe("OpenClawInstallPage", () => {
  beforeEach(() => {
    mockGoToStep.mockClear();
    mockInvoke.mockClear();
    mockListen.mockClear();
    mockListen.mockImplementation(() => Promise.resolve(vi.fn()));
    // Mock test_mirror_latency for auto-test on mount
    mockInvoke.mockResolvedValue(100);
    useInstallStore.setState({
      openclawInstallStatus: "idle",
      openclawInstallLogs: [],
      openclawVersion: null,
      selectedMirror: { name: "mirror.npmmirror", url: "https://registry.npmmirror.com", type: "npm" },
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

  it("navigates back to step 2", async () => {
    const user = userEvent.setup();
    renderWithRouter(<OpenClawInstallPage />);

    await user.click(screen.getByText("btn.prev"));
    expect(mockGoToStep).toHaveBeenCalledWith(2);
  });

  it("starts install on next click when idle", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValue({ version: "1.0.0" });
    renderWithRouter(<OpenClawInstallPage />);

    await user.click(screen.getByText("btn.next"));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("install_openclaw", {
        mirror: "https://registry.npmmirror.com",
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
    expect(mockListen).toHaveBeenCalledWith("install-progress", expect.any(Function));
    expect(mockListen).toHaveBeenCalledWith("install-log", expect.any(Function));
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
    mockInvoke.mockResolvedValue({ version: "1.0.0" });
    useInstallStore.setState({ openclawInstallStatus: "error" });
    renderWithRouter(<OpenClawInstallPage />);

    await user.click(screen.getByText("btn.next"));
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("install_openclaw", expect.any(Object));
    });
  });

  it("auto-navigates to step 4 on success", async () => {
    useInstallStore.setState({ openclawInstallStatus: "success" });
    renderWithRouter(<OpenClawInstallPage />);

    await waitFor(() => {
      expect(mockGoToStep).toHaveBeenCalledWith(4);
    });
  });

  it("selects a different mirror on click", async () => {
    const user = userEvent.setup();
    renderWithRouter(<OpenClawInstallPage />);

    await user.click(screen.getByText("mirror.official"));
    expect(useInstallStore.getState().selectedMirror?.url).toBe("https://registry.npmjs.org");
  });
});
