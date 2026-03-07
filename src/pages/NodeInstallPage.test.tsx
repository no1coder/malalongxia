import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen } from "@testing-library/react";
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
import NodeInstallPage from "./NodeInstallPage";
import { renderWithRouter } from "../test/render";

describe("NodeInstallPage", () => {
  beforeEach(() => {
    mockGoToStep.mockClear();
    mockInvoke.mockClear();
    mockListen.mockClear();
    mockListen.mockImplementation(() => Promise.resolve(vi.fn()));
    useInstallStore.setState({
      nodeVersion: null,
      nodeRequired: true,
      nodeInstallStatus: "idle",
      nodeInstallMethod: "nvm",
      nodeInstallLogs: [],
      selectedMirror: { name: "aliyun", url: "https://npmmirror.com/mirrors/node/", type: "node" },
    });
  });

  it("renders title and description", () => {
    renderWithRouter(<NodeInstallPage />);
    expect(screen.getByText("nodeInstall.title")).toBeInTheDocument();
    expect(screen.getByText("nodeInstall.description")).toBeInTheDocument();
  });

  it("shows already installed notice when not required", () => {
    useInstallStore.setState({ nodeRequired: false, nodeVersion: "v22.14.0" });
    renderWithRouter(<NodeInstallPage />);
    expect(screen.getByText(/nodeInstall.alreadyInstalled/)).toBeInTheDocument();
  });

  it("shows installation methods when required", () => {
    renderWithRouter(<NodeInstallPage />);
    expect(screen.getByText("nvm")).toBeInTheDocument();
    expect(screen.getByText("nodeInstall.directInstall")).toBeInTheDocument();
  });

  it("shows nvm as recommended", () => {
    renderWithRouter(<NodeInstallPage />);
    expect(screen.getByText("nodeInstall.recommended")).toBeInTheDocument();
  });

  it("selects direct method on click", async () => {
    const user = userEvent.setup();
    renderWithRouter(<NodeInstallPage />);

    await user.click(screen.getByText("nodeInstall.directInstall"));
    expect(useInstallStore.getState().nodeInstallMethod).toBe("direct");
  });

  it("renders mirror selection list", () => {
    renderWithRouter(<NodeInstallPage />);
    expect(screen.getByText("nodeInstall.mirrorSelect")).toBeInTheDocument();
    expect(screen.getByText("mirror.aliyun")).toBeInTheDocument();
  });

  it("navigates back to step 1", async () => {
    const user = userEvent.setup();
    renderWithRouter(<NodeInstallPage />);

    await user.click(screen.getByText("btn.prev"));
    expect(mockGoToStep).toHaveBeenCalledWith(1);
  });

  it("navigates to step 3 when nodeRequired is false", async () => {
    const user = userEvent.setup();
    useInstallStore.setState({ nodeRequired: false });
    renderWithRouter(<NodeInstallPage />);

    await user.click(screen.getByText("btn.next"));
    expect(mockGoToStep).toHaveBeenCalledWith(3);
  });

  it("starts install on next click when idle and required", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValue(undefined);
    renderWithRouter(<NodeInstallPage />);

    await user.click(screen.getByText("btn.next"));
    expect(mockInvoke).toHaveBeenCalledWith("install_node", {
      mirror: "https://npmmirror.com/mirrors/node/",
      method: "nvm",
    });
  });

  it("disables next button during install", () => {
    useInstallStore.setState({ nodeInstallStatus: "installing" });
    renderWithRouter(<NodeInstallPage />);

    const nextBtn = screen.getByText("nodeInstall.installing");
    expect(nextBtn).toBeDisabled();
  });

  it("disables back button during install", () => {
    useInstallStore.setState({ nodeInstallStatus: "installing" });
    renderWithRouter(<NodeInstallPage />);

    const backBtn = screen.getByText("btn.prev");
    expect(backBtn).toBeDisabled();
  });

  it("disables next when no mirror selected and required", () => {
    useInstallStore.setState({ selectedMirror: null });
    renderWithRouter(<NodeInstallPage />);

    const nextBtn = screen.getByText("btn.next");
    expect(nextBtn).toBeDisabled();
  });

  it("registers event listeners on mount", () => {
    renderWithRouter(<NodeInstallPage />);
    expect(mockListen).toHaveBeenCalledWith("install-progress", expect.any(Function));
    expect(mockListen).toHaveBeenCalledWith("install-log", expect.any(Function));
  });

  it("shows log entries when present", () => {
    useInstallStore.setState({
      nodeInstallLogs: [
        { timestamp: 1000, level: "info", message: "Downloading nvm..." },
        { timestamp: 2000, level: "error", message: "Connection failed" },
      ],
    });
    renderWithRouter(<NodeInstallPage />);

    expect(screen.getByText("Downloading nvm...")).toBeInTheDocument();
    expect(screen.getByText("Connection failed")).toBeInTheDocument();
  });

  it("allows retry after error", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValue(undefined);
    useInstallStore.setState({ nodeInstallStatus: "error" });
    renderWithRouter(<NodeInstallPage />);

    await user.click(screen.getByText("btn.next"));
    expect(mockInvoke).toHaveBeenCalled();
  });
});
