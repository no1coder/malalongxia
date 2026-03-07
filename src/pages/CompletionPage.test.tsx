import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const { mockInvoke } = vi.hoisted(() => ({
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

vi.mock("@tauri-apps/api/core", () => ({ invoke: mockInvoke }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));

import { useInstallStore } from "../stores/useInstallStore";
import CompletionPage from "./CompletionPage";
import { renderWithRouter } from "../test/render";

describe("CompletionPage", () => {
  beforeEach(() => {
    mockInvoke.mockClear();
    useInstallStore.setState({
      nodeVersion: "v22.14.0",
      nodeRequired: false,
      openclawVersion: "1.2.3",
      selectedProvider: {
        id: "zhipu",
        name: "apiConfig.zhipu",
        description: "desc",
        baseUrl: "https://open.bigmodel.cn/api/paas/v4",
        needsProxy: false,
        openclawProvider: "zai",
        defaultModel: "glm-5",
      },
    });
  });

  it("renders congratulations title", () => {
    renderWithRouter(<CompletionPage />);
    expect(screen.getByText("completion.congratulations")).toBeInTheDocument();
  });

  it("shows installation summary", () => {
    renderWithRouter(<CompletionPage />);
    expect(screen.getByText("Node.js")).toBeInTheDocument();
    expect(screen.getByText("OpenClaw")).toBeInTheDocument();
    expect(screen.getByText("v22.14.0")).toBeInTheDocument();
    expect(screen.getByText("1.2.3")).toBeInTheDocument();
  });

  it("shows not installed when openclawVersion is null", () => {
    useInstallStore.setState({ openclawVersion: null });
    renderWithRouter(<CompletionPage />);
    expect(screen.getByText("completion.notInstalled")).toBeInTheDocument();
  });

  it("renders launch button", () => {
    renderWithRouter(<CompletionPage />);
    expect(screen.getByText("completion.startUsing")).toBeInTheDocument();
  });

  it("calls launch_openclaw on launch click", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValue("http://localhost:18789");
    renderWithRouter(<CompletionPage />);

    await user.click(screen.getByText("completion.startUsing"));
    expect(mockInvoke).toHaveBeenCalledWith("launch_openclaw");
  });

  it("shows success message after launch", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValue("http://localhost:18789");
    renderWithRouter(<CompletionPage />);

    await user.click(screen.getByText("completion.startUsing"));

    await waitFor(() => {
      expect(screen.getByText(/completion.launchSuccess/)).toBeInTheDocument();
    });
  });

  it("calls onComplete after successful launch", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    const onComplete = vi.fn();
    mockInvoke.mockResolvedValue("http://localhost:18789");
    renderWithRouter(<CompletionPage onComplete={onComplete} />);

    await user.click(screen.getByText("completion.startUsing"));
    await waitFor(() => {
      expect(screen.getByText(/completion.launchSuccess/)).toBeInTheDocument();
    });

    vi.advanceTimersByTime(1500);
    expect(onComplete).toHaveBeenCalledTimes(1);
    vi.useRealTimers();
  });

  it("shows error message on launch failure", async () => {
    const user = userEvent.setup();
    mockInvoke.mockRejectedValue(new Error("Gateway failed"));
    renderWithRouter(<CompletionPage />);

    await user.click(screen.getByText("completion.startUsing"));

    await waitFor(() => {
      expect(screen.getByText("Error: Gateway failed")).toBeInTheDocument();
    });
  });

  it("renders benefits section", () => {
    renderWithRouter(<CompletionPage />);
    expect(screen.getByText("completion.benefitSkill")).toBeInTheDocument();
    expect(screen.getByText("completion.benefitSupport")).toBeInTheDocument();
    expect(screen.getByText("completion.benefitCommunity")).toBeInTheDocument();
    expect(screen.getByText("completion.benefitBeta")).toBeInTheDocument();
  });

  it("renders view tutorial button", () => {
    renderWithRouter(<CompletionPage />);
    expect(screen.getByText("completion.viewTutorial")).toBeInTheDocument();
  });
});
