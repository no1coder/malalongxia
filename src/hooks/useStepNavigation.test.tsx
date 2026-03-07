import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import type { ReactNode } from "react";

// Must mock before importing the hook
const mockNavigate = vi.fn();
vi.mock("react-router-dom", async () => {
  const actual = await vi.importActual<typeof import("react-router-dom")>("react-router-dom");
  return {
    ...actual,
    useNavigate: () => mockNavigate,
  };
});

import { useStepNavigation } from "./useStepNavigation";

function wrapper({ children }: { children: ReactNode }) {
  return <MemoryRouter>{children}</MemoryRouter>;
}

describe("useStepNavigation", () => {
  beforeEach(() => {
    mockNavigate.mockClear();
  });

  it("goToStep navigates to correct route", () => {
    const { result } = renderHook(() => useStepNavigation(), { wrapper });

    act(() => {
      result.current.goToStep(0);
    });
    expect(mockNavigate).toHaveBeenCalledWith("/");

    act(() => {
      result.current.goToStep(1);
    });
    expect(mockNavigate).toHaveBeenCalledWith("/env-check");

    act(() => {
      result.current.goToStep(4);
    });
    expect(mockNavigate).toHaveBeenCalledWith("/api-config");
  });

  it("goToStep clamps to valid range (lower bound)", () => {
    const { result } = renderHook(() => useStepNavigation(), { wrapper });

    act(() => {
      result.current.goToStep(-5);
    });
    expect(mockNavigate).toHaveBeenCalledWith("/");
  });

  it("goToStep clamps to valid range (upper bound)", () => {
    const { result } = renderHook(() => useStepNavigation(), { wrapper });

    act(() => {
      result.current.goToStep(100);
    });
    expect(mockNavigate).toHaveBeenCalledWith("/completion");
  });

  it("goNext increments step", () => {
    const { result } = renderHook(() => useStepNavigation(), { wrapper });

    act(() => {
      result.current.goNext(0);
    });
    expect(mockNavigate).toHaveBeenCalledWith("/env-check");

    act(() => {
      result.current.goNext(2);
    });
    expect(mockNavigate).toHaveBeenCalledWith("/openclaw-install");
  });

  it("goPrev decrements step", () => {
    const { result } = renderHook(() => useStepNavigation(), { wrapper });

    act(() => {
      result.current.goPrev(3);
    });
    expect(mockNavigate).toHaveBeenCalledWith("/node-install");
  });

  it("goPrev from 0 stays at 0", () => {
    const { result } = renderHook(() => useStepNavigation(), { wrapper });

    act(() => {
      result.current.goPrev(0);
    });
    expect(mockNavigate).toHaveBeenCalledWith("/");
  });

  it("exposes STEP_ROUTES array", () => {
    const { result } = renderHook(() => useStepNavigation(), { wrapper });
    expect(result.current.STEP_ROUTES).toHaveLength(6);
    expect(result.current.STEP_ROUTES[0]).toBe("/");
    expect(result.current.STEP_ROUTES[5]).toBe("/completion");
  });
});
