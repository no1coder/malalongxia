import clsx from "clsx";

interface ProgressBarProps {
  readonly progress: number;
  readonly label?: string;
  readonly status?: "active" | "success" | "error";
}

// Animated progress bar with percentage display and brand gradient
export function ProgressBar({
  progress,
  label,
  status = "active",
}: ProgressBarProps) {
  // Clamp progress to 0-100
  const clampedProgress = Math.max(0, Math.min(100, progress));

  return (
    <div className="progress-bar">
      {(label || true) && (
        <div className="progress-bar__header">
          {label && <span className="progress-bar__label">{label}</span>}
          <span className="progress-bar__percentage">
            {Math.round(clampedProgress)}%
          </span>
        </div>
      )}
      <div className="progress-bar__track">
        <div
          className={clsx("progress-bar__fill", `progress-bar__fill--${status}`)}
          style={{ width: `${clampedProgress}%` }}
        />
      </div>
    </div>
  );
}
