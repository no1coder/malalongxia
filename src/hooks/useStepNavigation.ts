import { useNavigate } from "react-router-dom";
import { useCallback } from "react";

// Route paths in step order, matching the route definitions in App.tsx
const STEP_ROUTES = [
  "/",
  "/env-check",
  "/node-install",
  "/openclaw-install",
  "/api-config",
  "/completion",
] as const;

export function useStepNavigation() {
  const navigate = useNavigate();

  const goToStep = useCallback(
    (step: number) => {
      const clamped = Math.max(0, Math.min(step, STEP_ROUTES.length - 1));
      navigate(STEP_ROUTES[clamped]);
    },
    [navigate]
  );

  const goNext = useCallback(
    (currentStep: number) => goToStep(currentStep + 1),
    [goToStep]
  );

  const goPrev = useCallback(
    (currentStep: number) => goToStep(currentStep - 1),
    [goToStep]
  );

  return { goToStep, goNext, goPrev, STEP_ROUTES };
}
