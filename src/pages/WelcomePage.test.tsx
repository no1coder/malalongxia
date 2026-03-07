import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

// Use vi.hoisted to create mock functions accessible in vi.mock factories
const { mockGoToStep } = vi.hoisted(() => ({
  mockGoToStep: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
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

vi.mock("@tauri-apps/api/app", () => ({
  getVersion: () => Promise.resolve("1.2.3"),
}));

import WelcomePage from "./WelcomePage";
import { renderWithRouter } from "../test/render";

describe("WelcomePage", () => {
  beforeEach(() => {
    mockGoToStep.mockClear();
  });

  it("renders title with lobster emoji and Chinese text", () => {
    renderWithRouter(<WelcomePage />);
    expect(screen.getByText("麻辣")).toBeInTheDocument();
    expect(screen.getByText("龙虾")).toBeInTheDocument();
  });

  it("renders subtitle", () => {
    renderWithRouter(<WelcomePage />);
    expect(screen.getByText("welcome.subtitle")).toBeInTheDocument();
  });

  it("renders version number", async () => {
    renderWithRouter(<WelcomePage />);
    // Version is loaded async from Tauri API (mocked to return "1.2.3")
    expect(await screen.findByText("v1.2.3")).toBeInTheDocument();
  });

  it("renders start button", () => {
    renderWithRouter(<WelcomePage />);
    expect(screen.getByText("btn.start")).toBeInTheDocument();
  });

  it("navigates to step 1 on start click", async () => {
    const user = userEvent.setup();
    renderWithRouter(<WelcomePage />);

    await user.click(screen.getByText("btn.start"));
    expect(mockGoToStep).toHaveBeenCalledWith(1);
  });
});
