import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen } from "@testing-library/react";
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

vi.mock("@tauri-apps/api/core", () => ({ invoke: mockInvoke }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));

import { useInstallStore } from "../stores/useInstallStore";
import ApiConfigPage from "./ApiConfigPage";
import { renderWithRouter } from "../test/render";

describe("ApiConfigPage", () => {
  beforeEach(() => {
    mockGoToStep.mockClear();
    mockInvoke.mockClear();
    useInstallStore.setState({
      selectedProvider: null,
      apiKey: "",
      apiBaseUrl: "",
      apiTestStatus: "idle",
    });
  });

  it("renders title and provider list", () => {
    renderWithRouter(<ApiConfigPage />);
    expect(screen.getByText("apiConfig.title")).toBeInTheDocument();
    expect(screen.getByText("apiConfig.zhipu")).toBeInTheDocument();
    expect(screen.getByText("apiConfig.qwen")).toBeInTheDocument();
    expect(screen.getByText("apiConfig.openai")).toBeInTheDocument();
  });

  it("shows form after selecting a provider", async () => {
    const user = userEvent.setup();
    renderWithRouter(<ApiConfigPage />);

    await user.click(screen.getByText("apiConfig.zhipu"));

    expect(screen.getByText("apiConfig.apiKeyInput")).toBeInTheDocument();
    expect(screen.getByText("apiConfig.baseUrlInput")).toBeInTheDocument();
  });

  it("disables test button when no API key", async () => {
    const user = userEvent.setup();
    renderWithRouter(<ApiConfigPage />);

    await user.click(screen.getByText("apiConfig.zhipu"));
    const testBtn = screen.getByText("apiConfig.testConnection");
    expect(testBtn).toBeDisabled();
  });

  it("enables test button when API key is entered", async () => {
    const user = userEvent.setup();
    renderWithRouter(<ApiConfigPage />);

    await user.click(screen.getByText("apiConfig.zhipu"));
    const input = screen.getByPlaceholderText("apiConfig.apiKeyPlaceholder");
    await user.type(input, "sk-test-123");

    const testBtn = screen.getByText("apiConfig.testConnection");
    expect(testBtn).not.toBeDisabled();
  });

  it("navigates back to step 3", async () => {
    const user = userEvent.setup();
    renderWithRouter(<ApiConfigPage />);

    await user.click(screen.getByText("btn.prev"));
    expect(mockGoToStep).toHaveBeenCalledWith(3);
  });

  it("skip button navigates to step 5", async () => {
    const user = userEvent.setup();
    renderWithRouter(<ApiConfigPage />);

    await user.click(screen.getByText("apiConfig.skip"));
    expect(mockGoToStep).toHaveBeenCalledWith(5);
  });

  it("next button disabled when provider selected but no key", async () => {
    const user = userEvent.setup();
    renderWithRouter(<ApiConfigPage />);

    await user.click(screen.getByText("apiConfig.zhipu"));
    const nextBtn = screen.getByText("btn.next");
    expect(nextBtn).toBeDisabled();
  });

  it("next button enabled when no provider selected", () => {
    renderWithRouter(<ApiConfigPage />);
    const nextBtn = screen.getByText("btn.next");
    expect(nextBtn).not.toBeDisabled();
  });

  it("shows recommended badge on qwen", () => {
    renderWithRouter(<ApiConfigPage />);
    expect(screen.getByText("apiConfig.recommended")).toBeInTheDocument();
  });

  it("shows proxy badge on openai and anthropic", () => {
    renderWithRouter(<ApiConfigPage />);
    const proxyBadges = screen.getAllByText("apiConfig.needsProxy");
    expect(proxyBadges.length).toBeGreaterThanOrEqual(2);
  });

  it("calls test_api_connection invoke on test connection", async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValue(undefined);
    renderWithRouter(<ApiConfigPage />);

    await user.click(screen.getByText("apiConfig.zhipu"));
    const input = screen.getByPlaceholderText("apiConfig.apiKeyPlaceholder");
    await user.type(input, "sk-test");

    await user.click(screen.getByText("apiConfig.testConnection"));

    expect(mockInvoke).toHaveBeenCalledWith("test_api_connection", {
      apiKey: "sk-test",
      baseUrl: "https://open.bigmodel.cn/api/paas/v4",
      model: "glm-5",
    });
  });
});
