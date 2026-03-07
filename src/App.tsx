import { BrowserRouter, Routes, Route, useLocation, useNavigate } from "react-router-dom";
import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Layout } from "./components/Layout";
import { UpdateBanner } from "./components/UpdateBanner";
import type { StepInfo } from "./components/StepIndicator";
import WelcomePage from "./pages/WelcomePage";
import EnvCheckPage from "./pages/EnvCheckPage";
import NodeInstallPage from "./pages/NodeInstallPage";
import OpenClawInstallPage from "./pages/OpenClawInstallPage";
import ApiConfigPage from "./pages/ApiConfigPage";
import CompletionPage from "./pages/CompletionPage";
import DashboardPage from "./pages/DashboardPage";
import FeishuConfigPage from "./pages/FeishuConfigPage";
import "./App.css";

// App update info from version.json
interface AppUpdateInfo {
  readonly has_update: boolean;
  readonly current_version: string;
  readonly latest_version: string;
  readonly download_url: string;
  readonly release_notes: string;
}

// Status returned from the Rust backend
interface OpenClawStatus {
  readonly installed: boolean;
  readonly running: boolean;
  readonly current_version: string | null;
  readonly latest_version: string | null;
  readonly needs_update: boolean;
  readonly gateway_url: string;
}

// Route definitions in step order (install wizard)
const STEP_ROUTES = [
  "/",
  "/env-check",
  "/node-install",
  "/openclaw-install",
  "/api-config",
  "/completion",
] as const;

// Step label translation keys
const STEP_LABELS = [
  "steps.welcome",
  "steps.envCheck",
  "steps.nodeInstall",
  "steps.openclawInstall",
  "steps.apiConfig",
  "steps.completion",
] as const;

// Derive current step index from pathname
function getStepIndex(pathname: string): number {
  const index = STEP_ROUTES.indexOf(pathname as (typeof STEP_ROUTES)[number]);
  return index === -1 ? 0 : index;
}

// Inner component that uses router hooks
function AppShell() {
  const location = useLocation();
  const navigate = useNavigate();
  const [checkState, setCheckState] = useState<"loading" | "install" | "dashboard">("loading");
  const [openclawStatus, setOpenclawStatus] = useState<OpenClawStatus | null>(null);
  const [appUpdate, setAppUpdate] = useState<AppUpdateInfo | null>(null);
  const [updateDismissed, setUpdateDismissed] = useState(false);

  // Only run the initial check once on first mount.
  // useRef guard prevents re-execution under React StrictMode's double-mount.
  const initialCheckDone = useRef(false);
  useEffect(() => {
    if (initialCheckDone.current) return;
    initialCheckDone.current = true;

    // Safety timeout: if backend hangs, fall through to install after 15s
    const timer = setTimeout(() => {
      console.warn("[App] check_openclaw_status timed out after 15s, falling back to install flow");
      setCheckState((prev) => (prev === "loading" ? "install" : prev));
    }, 15_000);

    (async () => {
      try {
        const status = await invoke<OpenClawStatus>("check_openclaw_status");
        clearTimeout(timer);
        setOpenclawStatus(status);
        if (status.installed) {
          setCheckState("dashboard");
          navigate("/dashboard", { replace: true });
        } else {
          setCheckState("install");
        }
      } catch (err) {
        console.warn("[App] check_openclaw_status failed:", err);
        clearTimeout(timer);
        setCheckState("install");
      }
    })();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Check for app updates on startup (non-blocking)
  useEffect(() => {
    invoke<AppUpdateInfo>("check_app_update")
      .then((info) => {
        if (info.has_update) {
          setAppUpdate(info);
        }
      })
      .catch((err) => {
        console.warn("[App] check_app_update failed:", err);
      });
  }, []);

  const handleReinstall = () => {
    setCheckState("install");
    navigate("/", { replace: true });
  };

  const handleReconfigureApi = () => {
    navigate("/reconfig-api", { replace: true });
  };

  const handleReconfigDone = () => {
    navigate("/dashboard", { replace: true });
  };

  const handleConfigureFeishu = () => {
    navigate("/feishu-config", { replace: true });
  };

  const currentStep = getStepIndex(location.pathname);
  const isDashboard = location.pathname === "/dashboard";
  const isReconfigApi = location.pathname === "/reconfig-api";
  const isFeishuConfig = location.pathname === "/feishu-config";

  // Build step info array with status derived from current step
  const steps: readonly StepInfo[] = useMemo(
    () =>
      STEP_LABELS.map((label, index) => ({
        label,
        status:
          index < currentStep
            ? "completed"
            : index === currentStep
              ? "active"
              : "pending",
      })),
    [currentStep],
  );

  const updateBanner = appUpdate && !updateDismissed ? (
    <UpdateBanner updateInfo={appUpdate} onDismiss={() => setUpdateDismissed(true)} />
  ) : null;

  // Loading state while checking
  if (checkState === "loading") {
    return (
      <div className="app-loading">
        <div className="app-loading-spinner" />
      </div>
    );
  }

  // Feishu config mode — no sidebar
  if (isFeishuConfig) {
    return (
      <Layout currentStep={-1} steps={[]} banner={updateBanner}>
        <FeishuConfigPage onDone={handleReconfigDone} />
      </Layout>
    );
  }

  // Reconfig API mode — no sidebar
  if (isReconfigApi) {
    return (
      <Layout currentStep={-1} steps={[]} banner={updateBanner}>
        <ApiConfigPage mode="reconfig" onDone={handleReconfigDone} />
      </Layout>
    );
  }

  // Dashboard mode — no sidebar steps
  if (isDashboard && openclawStatus) {
    return (
      <Layout currentStep={-1} steps={[]} banner={updateBanner}>
        <DashboardPage
          currentVersion={openclawStatus.current_version}
          latestVersion={openclawStatus.latest_version}
          needsUpdate={openclawStatus.needs_update}
          running={openclawStatus.running}
          gatewayUrl={openclawStatus.gateway_url}
          onReinstall={handleReinstall}
          onReconfigureApi={handleReconfigureApi}
          onConfigureFeishu={handleConfigureFeishu}
        />
      </Layout>
    );
  }

  return (
    <Layout
      currentStep={currentStep}
      steps={steps}
      banner={updateBanner}
    >
      <Routes>
        <Route path="/" element={<WelcomePage />} />
        <Route path="/env-check" element={<EnvCheckPage />} />
        <Route path="/node-install" element={<NodeInstallPage />} />
        <Route path="/openclaw-install" element={<OpenClawInstallPage />} />
        <Route path="/api-config" element={<ApiConfigPage />} />
        <Route path="/completion" element={<CompletionPage />} />
        <Route
          path="/dashboard"
          element={
            openclawStatus ? (
              <DashboardPage
                currentVersion={openclawStatus.current_version}
                latestVersion={openclawStatus.latest_version}
                needsUpdate={openclawStatus.needs_update}
                running={openclawStatus.running}
                gatewayUrl={openclawStatus.gateway_url}
                onReinstall={handleReinstall}
                onReconfigureApi={handleReconfigureApi}
                onConfigureFeishu={handleConfigureFeishu}
              />
            ) : null
          }
        />
      </Routes>
    </Layout>
  );
}

// Root App component wrapping with BrowserRouter
export default function App() {
  return (
    <BrowserRouter>
      <AppShell />
    </BrowserRouter>
  );
}
