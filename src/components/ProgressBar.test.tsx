import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { ProgressBar } from "./ProgressBar";

describe("ProgressBar", () => {
  it("renders percentage text", () => {
    render(<ProgressBar progress={42} />);
    expect(screen.getByText("42%")).toBeInTheDocument();
  });

  it("clamps progress above 100 to 100", () => {
    render(<ProgressBar progress={150} />);
    expect(screen.getByText("100%")).toBeInTheDocument();
  });

  it("clamps progress below 0 to 0", () => {
    render(<ProgressBar progress={-10} />);
    expect(screen.getByText("0%")).toBeInTheDocument();
  });

  it("rounds progress to integer", () => {
    render(<ProgressBar progress={33.7} />);
    expect(screen.getByText("34%")).toBeInTheDocument();
  });

  it("renders label when provided", () => {
    render(<ProgressBar progress={50} label="Downloading..." />);
    expect(screen.getByText("Downloading...")).toBeInTheDocument();
  });

  it("sets fill width matching clamped progress", () => {
    const { container } = render(<ProgressBar progress={75} />);
    const fill = container.querySelector(".progress-bar__fill");
    expect(fill).toHaveStyle({ width: "75%" });
  });

  it("applies status class to fill", () => {
    const { container } = render(<ProgressBar progress={50} status="success" />);
    const fill = container.querySelector(".progress-bar__fill");
    expect(fill?.classList.contains("progress-bar__fill--success")).toBe(true);
  });

  it("defaults to active status", () => {
    const { container } = render(<ProgressBar progress={50} />);
    const fill = container.querySelector(".progress-bar__fill");
    expect(fill?.classList.contains("progress-bar__fill--active")).toBe(true);
  });
});
