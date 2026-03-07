import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useInstallStore } from "../stores/useInstallStore";
import { useStepNavigation } from "../hooks/useStepNavigation";
import { useMirrorConfig } from "../hooks/useMirrorConfig";
import type { NodeVerifyResult } from "../types";
import { CheckCircle2, AlertTriangle, Loader2 } from "lucide-react";
import clsx from "clsx";
import "./NodeInstallPage.css";

export default function NodeInstallPage() {
  const { t } = useTranslation();
  const {
    nodeVersion,
    nodeRequired,
    nodeInstallStatus,
    nodeInstallMethod,
    nodeInstallLogs,
    selectedMirror,
    setNodeInstallMethod,
    setSelectedMirror,
    setNodeInstallStatus,
    addNodeInstallLog,
  } = useInstallStore();
  const { goToStep } = useStepNavigation();
  const { nodeMirrors, isLoading: mirrorsLoading } = useMirrorConfig();

  const [installPercent, setInstallPercent] = useState(0);
  const [installMessage, setInstallMessage] = useState("");
  const [verifyError, setVerifyError] = useState<string | null>(null);
  const isWindows = navigator.userAgent.includes("Windows");
  const progressRef = useRef<HTMLDivElement>(null);
  const logsRef = useRef<HTMLDivElement>(null);

  // Listen for install progress and log events from backend
  useEffect(() => {
    const unlisteners: (() => void)[] = [];

    listen<{ percent: number; message: string }>("node-install-progress", (event) => {
      setInstallPercent(event.payload.percent);
      setInstallMessage(event.payload.message);
    }).then((unlisten) => unlisteners.push(unlisten));

    listen<string>("node-install-log", (event) => {
      addNodeInstallLog({
        timestamp: Date.now(),
        level: "info",
        message: event.payload,
      });
    }).then((unlisten) => unlisteners.push(unlisten));

    return () => {
      unlisteners.forEach((fn) => fn());
    };
  }, [addNodeInstallLog]);

  // Start Node.js installation via Tauri backend
  const handleInstall = useCallback(async () => {
    if (!selectedMirror) return;

    setVerifyError(null);
    setNodeInstallStatus("installing");
    addNodeInstallLog({
      timestamp: Date.now(),
      level: "info",
      message: t("nodeInstall.installing"),
    });

    try {
      await invoke("install_node", {
        mirror: selectedMirror.url,
        method: nodeInstallMethod,
      });
      // Backend already verified node/npm in post_install_verify
      setNodeInstallStatus("success");
      addNodeInstallLog({
        timestamp: Date.now(),
        level: "info",
        message: t("nodeInstall.complete"),
      });
    } catch (err) {
      setNodeInstallStatus("error");
      addNodeInstallLog({
        timestamp: Date.now(),
        level: "error",
        message: String(err),
      });
    }
  }, [
    selectedMirror,
    nodeInstallMethod,
    setNodeInstallStatus,
    addNodeInstallLog,
    t,
  ]);

  const isInstalling = nodeInstallStatus === "installing";
  const isComplete = nodeInstallStatus === "success";
  const isFailed = nodeInstallStatus === "error";

  // Verify node/npm are available, then navigate to next step
  const verifyAndProceed = useCallback(async () => {
    setVerifyError(null);
    try {
      const result = await invoke<NodeVerifyResult>("verify_node_npm");
      if (result.node_available && result.npm_available) {
        goToStep(3);
      } else {
        const missing = [
          !result.node_available && "Node.js",
          !result.npm_available && "npm",
        ].filter(Boolean).join(" / ");
        setVerifyError(t("nodeInstall.verifyFailed", { missing }));
      }
    } catch (err) {
      setVerifyError(String(err));
    }
  }, [goToStep, t]);

  // "下一步" only verifies and navigates; install is a separate button
  const handleNext = useCallback(async () => {
    if (!nodeRequired || isComplete) {
      await verifyAndProceed();
    }
  }, [nodeRequired, isComplete, verifyAndProceed]);

  // Scroll to progress area when install starts
  useEffect(() => {
    if (isInstalling) {
      progressRef.current?.scrollIntoView({ behavior: "smooth", block: "start" });
    }
  }, [isInstalling]);

  // Auto-scroll logs to bottom when new entries arrive
  useEffect(() => {
    if (logsRef.current) {
      logsRef.current.scrollTop = logsRef.current.scrollHeight;
    }
  }, [nodeInstallLogs]);

  // Auto-verify and navigate on successful install
  useEffect(() => {
    if (isComplete) {
      verifyAndProceed();
    }
  }, [isComplete, verifyAndProceed]);

  const handleBack = () => {
    goToStep(1);
  };

  return (
    <div className="nodeinstall-page">
      <div className="nodeinstall-header">
        <h1>{t("nodeInstall.title")}</h1>
        <p>{t("nodeInstall.description")}</p>
      </div>

      <div className="nodeinstall-content">
        {/* Already installed notice */}
        {!nodeRequired && nodeVersion && (
          <div className="nodeinstall-skip">
            <CheckCircle2 />
            <span className="nodeinstall-skip-text">
              {t("nodeInstall.alreadyInstalled", { version: nodeVersion })}
            </span>
          </div>
        )}

        {/* Installation options when needed */}
        {nodeRequired && (
          <>
            {/* Method selection */}
            <div className="nodeinstall-methods">
              <div
                className={clsx(
                  "nodeinstall-method",
                  nodeInstallMethod === "direct" && "selected"
                )}
                onClick={() => !isInstalling && setNodeInstallMethod("direct")}
              >
                <div className="nodeinstall-method-name">
                  {t("nodeInstall.directInstall")}
                  <span className="nodeinstall-method-badge">
                    {t("nodeInstall.recommended")}
                  </span>
                </div>
                <div className="nodeinstall-method-desc">
                  {t("nodeInstall.directDesc")}
                </div>
              </div>
              {!isWindows && (
                <div
                  className={clsx(
                    "nodeinstall-method",
                    nodeInstallMethod === "nvm" && "selected"
                  )}
                  onClick={() => !isInstalling && setNodeInstallMethod("nvm")}
                >
                  <div className="nodeinstall-method-name">nvm</div>
                  <div className="nodeinstall-method-desc">
                    {t("nodeInstall.nvmDesc")}
                  </div>
                </div>
              )}
            </div>

            {/* Mirror selection */}
            <div className="nodeinstall-mirror-section">
              <h3>{t("nodeInstall.mirrorSelect")}</h3>
              {mirrorsLoading ? (
                <div className="nodeinstall-mirrors-loading">
                  <Loader2 className="spin" size={16} />
                  <span>{t("mirror.loadingConfig")}</span>
                </div>
              ) : (
                <div className="nodeinstall-mirror-list">
                  {nodeMirrors.map((mirror) => (
                    <div
                      key={mirror.url}
                      className={clsx(
                        "nodeinstall-mirror",
                        selectedMirror?.url === mirror.url && "selected"
                      )}
                      onClick={() => !isInstalling && setSelectedMirror(mirror)}
                    >
                      <div className="nodeinstall-mirror-radio" />
                      <span className="nodeinstall-mirror-name">
                        {t(mirror.name)}
                      </span>
                      <span className="nodeinstall-mirror-latency">
                        {mirror.latency != null
                          ? `${mirror.latency}ms`
                          : t("mirror.untested")}
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </div>

            {/* Progress bar */}
            {isInstalling && (
              <div ref={progressRef} className="nodeinstall-progress">
                {installMessage && (
                  <div className="nodeinstall-progress-status">{installMessage}</div>
                )}
                <div className="nodeinstall-progress-bar-container">
                  {installPercent > 0 ? (
                    <div
                      className="nodeinstall-progress-bar"
                      style={{ width: `${installPercent}%` }}
                    />
                  ) : (
                    <div className="nodeinstall-progress-bar indeterminate" />
                  )}
                </div>
                {installPercent > 0 && (
                  <div className="nodeinstall-progress-percent">{installPercent}%</div>
                )}
              </div>
            )}

            {/* Log viewer */}
            {nodeInstallLogs.length > 0 && (
              <div ref={logsRef} className="nodeinstall-logs">
                {nodeInstallLogs.map((log, i) => (
                  <div key={i} className={clsx("nodeinstall-log-entry", log.level)}>
                    {log.message}
                  </div>
                ))}
              </div>
            )}
          </>
        )}

        {/* Verification error */}
        {verifyError && (
          <div className="nodeinstall-verify-error">
            <AlertTriangle />
            <span>{verifyError}</span>
          </div>
        )}
      </div>

      {/* Navigation */}
      <div className="nodeinstall-actions">
        <button
          className="nodeinstall-btn nodeinstall-btn-secondary"
          onClick={handleBack}
          disabled={isInstalling}
        >
          {t("btn.prev")}
        </button>

        {/* Install / Retry button — only when installation is needed */}
        {nodeRequired && !isComplete && (
          <button
            className={clsx(
              "nodeinstall-btn nodeinstall-btn-primary",
              !isInstalling && selectedMirror && "btn-cta-glow"
            )}
            disabled={isInstalling || !selectedMirror}
            onClick={handleInstall}
          >
            {isInstalling
              ? t("nodeInstall.installing")
              : isFailed
                ? t("btn.retry")
                : t("nodeInstall.installBtn")}
          </button>
        )}

        {/* Next button — enabled after install success or when node not required */}
        <button
          className={clsx(
            "nodeinstall-btn nodeinstall-btn-primary",
            (!nodeRequired || isComplete) && "btn-cta-glow"
          )}
          disabled={nodeRequired && !isComplete}
          onClick={handleNext}
        >
          {t("btn.next")}
        </button>
      </div>
    </div>
  );
}
