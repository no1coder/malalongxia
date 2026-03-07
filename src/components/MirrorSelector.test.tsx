import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { Mirror } from "../types";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: "zh-CN", changeLanguage: vi.fn() },
  }),
}));

import { MirrorSelector } from "./MirrorSelector";

const MOCK_MIRRORS: Mirror[] = [
  { name: "Aliyun", url: "https://npmmirror.com", type: "npm", latency: 50 },
  { name: "Tencent", url: "https://mirrors.cloud.tencent.com", type: "npm", latency: 120 },
  { name: "Untested", url: "https://example.com", type: "npm" },
];

describe("MirrorSelector", () => {
  it("renders all mirror names", () => {
    render(
      <MirrorSelector
        mirrors={MOCK_MIRRORS}
        selected=""
        onSelect={vi.fn()}
        onTest={vi.fn()}
      />
    );
    expect(screen.getByText("Aliyun")).toBeInTheDocument();
    expect(screen.getByText("Tencent")).toBeInTheDocument();
    expect(screen.getByText("Untested")).toBeInTheDocument();
  });

  it("renders mirror URLs", () => {
    render(
      <MirrorSelector
        mirrors={MOCK_MIRRORS}
        selected=""
        onSelect={vi.fn()}
        onTest={vi.fn()}
      />
    );
    expect(screen.getByText("https://npmmirror.com")).toBeInTheDocument();
  });

  it("shows latency for tested mirrors", () => {
    render(
      <MirrorSelector
        mirrors={MOCK_MIRRORS}
        selected=""
        onSelect={vi.fn()}
        onTest={vi.fn()}
      />
    );
    expect(screen.getByText("50ms")).toBeInTheDocument();
    expect(screen.getByText("120ms")).toBeInTheDocument();
  });

  it("shows untested label for untested mirrors", () => {
    render(
      <MirrorSelector
        mirrors={MOCK_MIRRORS}
        selected=""
        onSelect={vi.fn()}
        onTest={vi.fn()}
      />
    );
    expect(screen.getByText("mirror.untested")).toBeInTheDocument();
  });

  it("calls onSelect when clicking a mirror", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(
      <MirrorSelector
        mirrors={MOCK_MIRRORS}
        selected=""
        onSelect={onSelect}
        onTest={vi.fn()}
      />
    );

    await user.click(screen.getByText("Aliyun"));
    expect(onSelect).toHaveBeenCalledWith("https://npmmirror.com");
  });

  it("calls onTest when clicking speed test button", async () => {
    const user = userEvent.setup();
    const onTest = vi.fn();
    render(
      <MirrorSelector
        mirrors={MOCK_MIRRORS}
        selected=""
        onSelect={vi.fn()}
        onTest={onTest}
      />
    );

    await user.click(screen.getByText("mirror.testSpeed"));
    expect(onTest).toHaveBeenCalledTimes(1);
  });

  it("disables test button when testing", () => {
    render(
      <MirrorSelector
        mirrors={MOCK_MIRRORS}
        selected=""
        onSelect={vi.fn()}
        onTest={vi.fn()}
        testing={true}
      />
    );
    const testBtn = screen.getByText("mirror.testing").closest("button");
    expect(testBtn).toBeDisabled();
  });

  it("highlights selected mirror", () => {
    const { container } = render(
      <MirrorSelector
        mirrors={MOCK_MIRRORS}
        selected="https://npmmirror.com"
        onSelect={vi.fn()}
        onTest={vi.fn()}
      />
    );
    const selectedItem = container.querySelector(".mirror-selector__item--selected");
    expect(selectedItem).toBeInTheDocument();
  });
});
