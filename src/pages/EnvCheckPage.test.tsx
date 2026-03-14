import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const { mockGoToStep, mockInvoke } = vi.hoisted(() => ({
  mockGoToStep: vi.fn(),
  mockInvoke: vi.fn(),
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

vi.mock("@tauri-apps/api/core", () => ({
  invoke: mockInvoke,
}));

import { useInstallStore } from "../stores/useInstallStore";
import EnvCheckPage from "./EnvCheckPage";
import { renderWithRouter } from "../test/render";

describe("EnvCheckPage", () => {
  beforeEach(() => {
    mockGoToStep.mockClear();
    mockInvoke.mockClear();
    useInstallStore.setState({
      envChecks: [],
      envCheckComplete: false,
      nodeRequired: false,
      nodeVersion: null,
      npmVersion: null,
      diskSpaceOk: true,
      osType: "unknown",
      osVersion: "",
    });
  });

  it("renders title and description", () => {
    useInstallStore.setState({
      envChecks: [{ id: "os", name: "envCheck.os", status: "pending", detail: "" }],
    });
    renderWithRouter(<EnvCheckPage />);
    expect(screen.getByText("envCheck.title")).toBeInTheDocument();
    expect(screen.getByText("envCheck.description")).toBeInTheDocument();
  });

  it("renders check items when populated", () => {
    useInstallStore.setState({
      envChecks: [
        { id: "os", name: "envCheck.os", status: "passed", detail: "macOS 14.0" },
        { id: "node", name: "envCheck.node", status: "passed", detail: "v22.22.0" },
      ],
      envCheckComplete: true,
    });
    renderWithRouter(<EnvCheckPage />);
    expect(screen.getByText("envCheck.os")).toBeInTheDocument();
    expect(screen.getByText("envCheck.node")).toBeInTheDocument();
  });

  it("shows summary when all checks complete with no issues", () => {
    useInstallStore.setState({
      envChecks: [
        { id: "os", name: "envCheck.os", status: "passed", detail: "macOS" },
        { id: "node", name: "envCheck.node", status: "passed", detail: "v22" },
      ],
      envCheckComplete: true,
    });
    renderWithRouter(<EnvCheckPage />);
    expect(screen.getByText("envCheck.allPassed")).toBeInTheDocument();
  });

  it("shows failure summary when checks have failures", () => {
    useInstallStore.setState({
      envChecks: [
        { id: "os", name: "envCheck.os", status: "passed", detail: "macOS" },
        { id: "node", name: "envCheck.node", status: "failed", detail: "Not installed" },
      ],
      envCheckComplete: true,
    });
    renderWithRouter(<EnvCheckPage />);
    expect(screen.getByText("envCheck.hasFailed")).toBeInTheDocument();
  });

  it("disables next button when checks incomplete", () => {
    useInstallStore.setState({
      envChecks: [{ id: "os", name: "envCheck.os", status: "checking", detail: "" }],
      envCheckComplete: false,
    });
    renderWithRouter(<EnvCheckPage />);
    const nextBtn = screen.getByText("btn.next");
    expect(nextBtn).toBeDisabled();
  });

  it("enables next button when checks complete with no blocking failures", () => {
    useInstallStore.setState({
      envChecks: [
        { id: "os", name: "envCheck.os", status: "passed", detail: "macOS" },
        { id: "network", name: "envCheck.network", status: "failed", detail: "offline" },
      ],
      envCheckComplete: true,
    });
    renderWithRouter(<EnvCheckPage />);
    const nextBtn = screen.getByText("btn.next");
    expect(nextBtn).not.toBeDisabled();
  });

  it("disables next button when non-network check fails", () => {
    useInstallStore.setState({
      envChecks: [{ id: "os", name: "envCheck.os", status: "failed", detail: "Unsupported" }],
      envCheckComplete: true,
    });
    renderWithRouter(<EnvCheckPage />);
    const nextBtn = screen.getByText("btn.next");
    expect(nextBtn).toBeDisabled();
  });

  it("navigates to step 2 when nodeRequired", async () => {
    const user = userEvent.setup();
    useInstallStore.setState({
      envChecks: [{ id: "os", name: "envCheck.os", status: "passed", detail: "macOS" }],
      envCheckComplete: true,
      nodeRequired: true,
    });
    renderWithRouter(<EnvCheckPage />);

    await user.click(screen.getByText("btn.next"));
    expect(mockGoToStep).toHaveBeenCalledWith(2);
  });

  it("navigates to step 3 when node not required", async () => {
    const user = userEvent.setup();
    useInstallStore.setState({
      envChecks: [{ id: "os", name: "envCheck.os", status: "passed", detail: "macOS" }],
      envCheckComplete: true,
      nodeRequired: false,
    });
    renderWithRouter(<EnvCheckPage />);

    await user.click(screen.getByText("btn.next"));
    expect(mockGoToStep).toHaveBeenCalledWith(3);
  });

  it("navigates back to step 0", async () => {
    const user = userEvent.setup();
    useInstallStore.setState({
      envChecks: [{ id: "os", name: "envCheck.os", status: "pending", detail: "" }],
    });
    renderWithRouter(<EnvCheckPage />);

    await user.click(screen.getByText("btn.prev"));
    expect(mockGoToStep).toHaveBeenCalledWith(0);
  });

  it("shows retry button when checks complete", () => {
    useInstallStore.setState({
      envChecks: [{ id: "os", name: "envCheck.os", status: "passed", detail: "macOS" }],
      envCheckComplete: true,
    });
    renderWithRouter(<EnvCheckPage />);
    expect(screen.getByText("btn.retry")).toBeInTheDocument();
  });

  it("runs checks on mount when envChecks empty", async () => {
    mockInvoke.mockResolvedValue({ status: "passed", detail: "OK", data: {} });
    useInstallStore.setState({ envChecks: [] });

    renderWithRouter(<EnvCheckPage />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalled();
    });
  });
});
