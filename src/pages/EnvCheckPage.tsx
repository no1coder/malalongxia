import { useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { useInstallStore } from "../stores/useInstallStore";
import { useStepNavigation } from "../hooks/useStepNavigation";
import type { CheckStatus, EnvCheckItem } from "../stores/useInstallStore";
import {
  Monitor,
  Cpu,
  Package,
  GitBranch,
  Wifi,
  HardDrive,
  Loader2,
  CheckCircle2,
  AlertTriangle,
  XCircle,
  Circle,
} from "lucide-react";
import clsx from "clsx";
import "./EnvCheckPage.css";

// Timeout duration for each check (ms)
const CHECK_TIMEOUT_MS = 10_000;

// Map check item id to its icon
const CHECK_ICONS: Record<string, React.ElementType> = {
  os: Monitor,
  node: Cpu,
  npm: Package,
  git: GitBranch,
  network: Wifi,
  disk: HardDrive,
};

// Map status to status icon component
const STATUS_ICONS: Record<CheckStatus, React.ElementType> = {
  pending: Circle,
  checking: Loader2,
  passed: CheckCircle2,
  warning: AlertTriangle,
  failed: XCircle,
};

// Map status to i18n key for badge label
const STATUS_LABEL_KEYS: Record<CheckStatus, string> = {
  pending: "envCheck.statusPending",
  checking: "envCheck.statusChecking",
  passed: "envCheck.statusPass",
  warning: "envCheck.statusWarning",
  failed: "envCheck.statusFail",
};

// Initial check items before running detection
const INITIAL_CHECKS: readonly EnvCheckItem[] = [
  { id: "os", name: "envCheck.os", status: "pending", detail: "" },
  { id: "node", name: "envCheck.node", status: "pending", detail: "" },
  { id: "npm", name: "envCheck.npm", status: "pending", detail: "" },
  { id: "git", name: "envCheck.git", status: "pending", detail: "" },
  { id: "network", name: "envCheck.network", status: "pending", detail: "" },
  { id: "disk", name: "envCheck.disk", status: "pending", detail: "" },
];

// Create a timeout promise that resolves with a warning result
function createTimeoutPromise(
  timeoutLabel: string
): Promise<{ status: CheckStatus; detail: string; data?: Record<string, unknown> }> {
  return new Promise((resolve) => {
    setTimeout(() => {
      resolve({ status: "warning" as CheckStatus, detail: timeoutLabel });
    }, CHECK_TIMEOUT_MS);
  });
}

export default function EnvCheckPage() {
  const { t } = useTranslation();
  const {
    envChecks,
    envCheckComplete,
    nodeRequired,
    setEnvChecks,
    updateEnvCheck,
    setEnvCheckComplete,
    setOsInfo,
    setNodeVersion,
    setNodeRequired,
    setNpmVersion,
    setDiskSpaceOk,
  } = useInstallStore();
  const { goToStep } = useStepNavigation();

  // Run a single check item with timeout protection
  const runCheck = useCallback(
    async (id: string) => {
      updateEnvCheck(id, "checking", t("envCheck.checking"));

      try {
        // Race the actual invoke against a timeout
        const result = await Promise.race([
          invoke<{
            status: CheckStatus;
            detail: string;
            data?: Record<string, unknown>;
          }>("check_environment", { checkId: id }),
          createTimeoutPromise(t("envCheck.timeout")),
        ]);

        // Use the detail string returned by backend directly
        updateEnvCheck(id, result.status, result.detail);

        // Store specific results in global state
        if (id === "os" && result.data) {
          setOsInfo(
            result.data.osType as "macos" | "windows" | "linux" | "unknown",
            result.data.osVersion as string
          );
        }
        if (id === "node") {
          const version = (result.data?.version as string | null) ?? null;
          setNodeVersion(version);
          // Node is required if: no version found, check failed, OR check timed out/warned
          setNodeRequired(!version || result.status === "failed" || result.status === "warning");
        }
        if (id === "npm") {
          setNpmVersion((result.data?.version as string | null) ?? null);
        }
        if (id === "disk") {
          setDiskSpaceOk(result.status !== "failed");
        }
      } catch {
        updateEnvCheck(id, "warning", t("envCheck.checkFailed"));
        // If node/npm check threw, assume they are not available
        if (id === "node") {
          setNodeVersion(null);
          setNodeRequired(true);
        }
        if (id === "npm") {
          setNpmVersion(null);
        }
      }
    },
    [
      updateEnvCheck,
      setOsInfo,
      setNodeVersion,
      setNodeRequired,
      setNpmVersion,
      setDiskSpaceOk,
      t,
    ]
  );

  // Run all checks in parallel for faster results
  const runAllChecks = useCallback(async () => {
    setEnvChecks(INITIAL_CHECKS);
    setEnvCheckComplete(false);

    await Promise.allSettled(INITIAL_CHECKS.map((check) => runCheck(check.id)));

    setEnvCheckComplete(true);
  }, [runCheck, setEnvChecks, setEnvCheckComplete]);

  // Auto-start checks on mount
  useEffect(() => {
    if (envChecks.length === 0) {
      runAllChecks();
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Count results by status
  const passedCount = envChecks.filter((c) => c.status === "passed").length;
  const warningCount = envChecks.filter((c) => c.status === "warning").length;
  const failedCount = envChecks.filter((c) => c.status === "failed").length;
  // node and npm failures are not blocking — next step installs them
  const NON_BLOCKING_CHECKS = new Set(["network", "node", "npm"]);
  const hasBlockingFailure = envChecks.some(
    (c) => c.status === "failed" && !NON_BLOCKING_CHECKS.has(c.id)
  );

  // Determine summary message
  const getSummaryMessage = (): string => {
    if (failedCount === 0 && warningCount === 0) return t("envCheck.allPassed");
    if (failedCount > 0) return t("envCheck.hasFailed");
    return t("envCheck.hasWarnings");
  };

  const handleNext = () => {
    // If Node.js needs install, go to step 2 (NodeInstallPage)
    // Otherwise skip to step 3 (OpenClawInstallPage)
    goToStep(nodeRequired ? 2 : 3);
  };

  const handleBack = () => {
    goToStep(0);
  };

  // Progress bar percentage
  const completedCount = envChecks.filter(
    (c) => c.status !== "pending" && c.status !== "checking"
  ).length;
  const progressPercent =
    envChecks.length > 0 ? Math.round((completedCount / envChecks.length) * 100) : 0;

  return (
    <div className="envcheck-page">
      <div className="envcheck-header">
        <h1>{t("envCheck.title")}</h1>
        <p>{t("envCheck.description")}</p>
      </div>

      {/* Progress bar */}
      {!envCheckComplete && (
        <div className="envcheck-progress">
          <div className="envcheck-progress-bar">
            <div
              className="envcheck-progress-fill"
              style={{ width: `${progressPercent}%` }}
            />
          </div>
          <div className="envcheck-progress-text">
            {completedCount}/{envChecks.length}
          </div>
        </div>
      )}

      {/* Check items list */}
      <div className="envcheck-list">
        {envChecks.map((check) => {
          const ItemIcon = CHECK_ICONS[check.id] ?? Circle;
          const StatusIcon = STATUS_ICONS[check.status];
          return (
            <div
              key={check.id}
              className={clsx("envcheck-item", check.status)}
            >
              {/* Left: status icon */}
              <div className={clsx("envcheck-icon", check.status)}>
                <StatusIcon />
              </div>

              {/* Middle: name + detail */}
              <div className="envcheck-info">
                <div className="envcheck-name">
                  <ItemIcon
                    style={{
                      width: 16,
                      height: 16,
                      marginRight: 6,
                      verticalAlign: "middle",
                    }}
                  />
                  {t(check.name)}
                </div>
                {check.detail && (
                  <div className={clsx("envcheck-detail", check.status)}>
                    {check.detail}
                  </div>
                )}
              </div>

              {/* Right: status badge */}
              <div className={clsx("envcheck-badge", check.status)}>
                {t(STATUS_LABEL_KEYS[check.status])}
              </div>
            </div>
          );
        })}
      </div>

      {/* Summary after all checks complete */}
      {envCheckComplete && (
        <div
          className={clsx("envcheck-summary", {
            "all-passed": failedCount === 0 && warningCount === 0,
            "has-issues": failedCount > 0 || warningCount > 0,
          })}
        >
          {/* Summary icon */}
          <div className="envcheck-summary-icon">
            {failedCount === 0 && warningCount === 0 ? (
              <CheckCircle2 />
            ) : failedCount > 0 ? (
              <XCircle />
            ) : (
              <AlertTriangle />
            )}
          </div>

          {/* Summary message */}
          <div className="envcheck-summary-text">
            <h3>{getSummaryMessage()}</h3>

            {/* Stat badges */}
            <div className="envcheck-summary-stats">
              <span className="envcheck-stat passed">
                <CheckCircle2 size={12} />
                {t("envCheck.statPassed", { count: passedCount })}
              </span>
              {warningCount > 0 && (
                <span className="envcheck-stat warning">
                  <AlertTriangle size={12} />
                  {t("envCheck.statWarning", { count: warningCount })}
                </span>
              )}
              {failedCount > 0 && (
                <span className="envcheck-stat failed">
                  <XCircle size={12} />
                  {t("envCheck.statFailed", { count: failedCount })}
                </span>
              )}
            </div>
          </div>

          {/* Network failure warning */}
          {envChecks.some(
            (c) => c.id === "network" && (c.status === "failed" || c.status === "warning")
          ) && (
            <p className="envcheck-network-warning">
              <Wifi size={12} />
              {t("envCheck.networkWarning")}
            </p>
          )}
        </div>
      )}

      {/* Action buttons - fixed at bottom */}
      <div className="envcheck-actions">
        <button
          className="envcheck-btn envcheck-btn-secondary"
          onClick={handleBack}
        >
          {t("btn.prev")}
        </button>
        <div style={{ display: "flex", gap: "0.75rem" }}>
          {envCheckComplete && (
            <button
              className="envcheck-btn envcheck-btn-secondary"
              onClick={runAllChecks}
            >
              {t("btn.retry")}
            </button>
          )}
          <button
            className={clsx(
              "envcheck-btn envcheck-btn-primary",
              envCheckComplete && !hasBlockingFailure && "btn-cta-glow"
            )}
            disabled={!envCheckComplete || hasBlockingFailure}
            onClick={handleNext}
          >
            {t("btn.next")}
          </button>
        </div>
      </div>
    </div>
  );
}
