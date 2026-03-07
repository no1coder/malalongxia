import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useInstallStore } from "../stores/useInstallStore";
import { useStepNavigation } from "../hooks/useStepNavigation";
import type { Mirror } from "../types";
import { CheckCircle2 } from "lucide-react";
import clsx from "clsx";
import "./NodeInstallPage.css";

// Mirror sources for Node.js downloads
const NODE_MIRRORS: readonly Mirror[] = [
  { name: "mirror.aliyun", url: "https://npmmirror.com/mirrors/node/", type: "node" },
  { name: "mirror.tencent", url: "https://mirrors.cloud.tencent.com/nodejs-release/", type: "node" },
  { name: "mirror.tsinghua", url: "https://mirrors.tuna.tsinghua.edu.cn/nodejs-release/", type: "node" },
  { name: "mirror.huawei", url: "https://repo.huaweicloud.com/nodejs/", type: "node" },
];

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

  const [installPercent, setInstallPercent] = useState(0);
  const [installMessage, setInstallMessage] = useState("");
  const progressRef = useRef<HTMLDivElement>(null);
  const logsRef = useRef<HTMLDivElement>(null);

  // Listen for install progress and log events from backend
  useEffect(() => {
    const unlisteners: (() => void)[] = [];

    listen<{ percent: number; message: string }>("install-progress", (event) => {
      setInstallPercent(event.payload.percent);
      setInstallMessage(event.payload.message);
    }).then((unlisten) => unlisteners.push(unlisten));

    listen<string>("install-log", (event) => {
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

  // "下一步" triggers install if needed, navigates if complete or not required
  const handleNext = useCallback(async () => {
    if (!nodeRequired || isComplete) {
      goToStep(3);
      return;
    }
    // Allow retry after error
    if ((nodeInstallStatus === "idle" || nodeInstallStatus === "error") && selectedMirror) {
      setNodeInstallStatus("idle");
      await handleInstall();
    }
  }, [nodeRequired, isComplete, nodeInstallStatus, selectedMirror, handleInstall, goToStep, setNodeInstallStatus]);

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

  // Auto-navigate on success
  useEffect(() => {
    if (isComplete) {
      goToStep(3);
    }
  }, [isComplete, goToStep]);

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
                  nodeInstallMethod === "nvm" && "selected"
                )}
                onClick={() => !isInstalling && setNodeInstallMethod("nvm")}
              >
                <div className="nodeinstall-method-name">
                  nvm
                  <span className="nodeinstall-method-badge">
                    {t("nodeInstall.recommended")}
                  </span>
                </div>
                <div className="nodeinstall-method-desc">
                  {t("nodeInstall.nvmDesc")}
                </div>
              </div>
              <div
                className={clsx(
                  "nodeinstall-method",
                  nodeInstallMethod === "direct" && "selected"
                )}
                onClick={() => !isInstalling && setNodeInstallMethod("direct")}
              >
                <div className="nodeinstall-method-name">
                  {t("nodeInstall.directInstall")}
                </div>
                <div className="nodeinstall-method-desc">
                  {t("nodeInstall.directDesc")}
                </div>
              </div>
            </div>

            {/* Mirror selection */}
            <div className="nodeinstall-mirror-section">
              <h3>{t("nodeInstall.mirrorSelect")}</h3>
              <div className="nodeinstall-mirror-list">
                {NODE_MIRRORS.map((mirror) => (
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
        <button
          className={clsx(
            "nodeinstall-btn nodeinstall-btn-primary",
            !isInstalling && (selectedMirror || !nodeRequired) && "btn-cta-glow"
          )}
          disabled={isInstalling || (nodeRequired && !isComplete && !selectedMirror)}
          onClick={handleNext}
        >
          {isInstalling
            ? t("nodeInstall.installing")
            : t("btn.next")}
        </button>
      </div>
    </div>
  );
}
