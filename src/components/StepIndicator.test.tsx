import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import type { StepInfo } from "./StepIndicator";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: "zh-CN", changeLanguage: vi.fn() },
  }),
}));

import { StepIndicator } from "./StepIndicator";

const MOCK_STEPS: StepInfo[] = [
  { label: "steps.welcome", status: "completed" },
  { label: "steps.envCheck", status: "active" },
  { label: "steps.nodeInstall", status: "pending" },
];

describe("StepIndicator", () => {
  it("renders all step labels", () => {
    render(
      <StepIndicator currentStep={1} totalSteps={3} steps={MOCK_STEPS} />
    );
    expect(screen.getByText("steps.welcome")).toBeInTheDocument();
    expect(screen.getByText("steps.envCheck")).toBeInTheDocument();
    expect(screen.getByText("steps.nodeInstall")).toBeInTheDocument();
  });

  it("applies active class to current step", () => {
    const { container } = render(
      <StepIndicator currentStep={1} totalSteps={3} steps={MOCK_STEPS} />
    );
    const dots = container.querySelectorAll(".step-indicator__dot");
    expect(dots[1].classList.contains("step-indicator__dot--active")).toBe(true);
  });

  it("applies completed class to completed steps", () => {
    const { container } = render(
      <StepIndicator currentStep={1} totalSteps={3} steps={MOCK_STEPS} />
    );
    const dots = container.querySelectorAll(".step-indicator__dot");
    expect(dots[0].classList.contains("step-indicator__dot--completed")).toBe(true);
  });

  it("applies pending class to pending steps", () => {
    const { container } = render(
      <StepIndicator currentStep={1} totalSteps={3} steps={MOCK_STEPS} />
    );
    const dots = container.querySelectorAll(".step-indicator__dot");
    expect(dots[2].classList.contains("step-indicator__dot--pending")).toBe(true);
  });

  it("shows step numbers for non-completed steps", () => {
    render(
      <StepIndicator currentStep={1} totalSteps={3} steps={MOCK_STEPS} />
    );
    // Active step shows number 2
    expect(screen.getByText("2")).toBeInTheDocument();
    // Pending step shows number 3
    expect(screen.getByText("3")).toBeInTheDocument();
  });

  it("renders connecting lines between steps (except last)", () => {
    const { container } = render(
      <StepIndicator currentStep={1} totalSteps={3} steps={MOCK_STEPS} />
    );
    const lines = container.querySelectorAll(".step-indicator__line");
    // 3 steps => 2 connecting lines
    expect(lines).toHaveLength(2);
  });

  it("applies completed class to lines for completed steps", () => {
    const { container } = render(
      <StepIndicator currentStep={1} totalSteps={3} steps={MOCK_STEPS} />
    );
    const lines = container.querySelectorAll(".step-indicator__line");
    expect(lines[0].classList.contains("step-indicator__line--completed")).toBe(true);
    expect(lines[1].classList.contains("step-indicator__line--completed")).toBe(false);
  });
});
