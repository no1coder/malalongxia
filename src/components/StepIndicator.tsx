import { Check } from "lucide-react";
import { useTranslation } from "react-i18next";
import clsx from "clsx";

export interface StepInfo {
  readonly label: string;
  readonly status: "pending" | "active" | "completed" | "error" | "skipped";
}

interface StepIndicatorProps {
  readonly currentStep: number;
  readonly totalSteps: number;
  readonly steps: readonly StepInfo[];
}

// Vertical step list for the sidebar navigation (macOS installer style)
export function StepIndicator({
  steps,
}: StepIndicatorProps) {
  const { t } = useTranslation();

  return (
    <nav className="step-indicator">
      {steps.map((step, index) => (
        <div key={step.label} className="step-indicator__item">
          {/* Vertical connecting line (skip for last step) */}
          {index < steps.length - 1 && (
            <div
              className={clsx("step-indicator__line", {
                "step-indicator__line--completed": step.status === "completed",
              })}
            />
          )}

          {/* Step circle with number or checkmark */}
          <div
            className={clsx("step-indicator__dot", {
              "step-indicator__dot--active": step.status === "active",
              "step-indicator__dot--completed": step.status === "completed",
              "step-indicator__dot--error": step.status === "error",
              "step-indicator__dot--pending": step.status === "pending",
            })}
          >
            {step.status === "completed" ? (
              <Check size={14} strokeWidth={3} />
            ) : (
              <span className="step-indicator__number">{index + 1}</span>
            )}
          </div>

          {/* Step label */}
          <span
            className={clsx("step-indicator__label", {
              "step-indicator__label--active": step.status === "active",
              "step-indicator__label--completed": step.status === "completed",
            })}
          >
            {t(step.label)}
          </span>
        </div>
      ))}
    </nav>
  );
}
