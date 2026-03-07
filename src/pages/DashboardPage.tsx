import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  Rocket,
  RefreshCw,
  ArrowUpCircle,
  Settings,
  Sparkles,
  CheckCircle2,
  AlertCircle,
  RotateCcw,
  Globe,
  Circle,
  Loader2,
  Square,
  Stethoscope,
  Wrench,
  MessageSquare,
} from "lucide-react";
import "./DashboardPage.css";

const TIPS_URL = "https://malalongxia.com/tips.html";

interface DashboardPageProps {
  readonly currentVersion: string | null;
  readonly latestVersion: string | null;
  readonly needsUpdate: boolean;
  readonly running: boolean;
  readonly gatewayUrl: string;
  readonly onReinstall: () => void;
  readonly onReconfigureApi: () => void;
  readonly onConfigureFeishu: () => void;
}

interface OpenClawStatus {
  readonly installed: boolean;
  readonly running: boolean;
  readonly current_version: string | null;
  readonly latest_version: string | null;
  readonly needs_update: boolean;
  readonly gateway_url: string;
}

export default function DashboardPage({
  currentVersion,
  latestVersion,
  needsUpdate,
  running: initialRunning,
  gatewayUrl,
  onReinstall,
  onReconfigureApi,
  onConfigureFeishu,
}: DashboardPageProps) {
  const { t } = useTranslation();

  // Gateway state
  const [isRunning, setIsRunning] = useState(initialRunning);
  const [checking, setChecking] = useState(!initialRunning);

  // Launch
  const [launchStatus, setLaunchStatus] = useState<"idle" | "launching" | "success" | "error">("idle");
  const [launchMessage, setLaunchMessage] = useState<string | null>(null);

  // Stop
  const [stopStatus, setStopStatus] = useState<"idle" | "stopping" | "success" | "error">("idle");

  // Restart
  const [restartStatus, setRestartStatus] = useState<"idle" | "restarting" | "success" | "error">("idle");

  // Update
  const [updateStatus, setUpdateStatus] = useState<"idle" | "updating" | "success" | "error">("idle");
  const [updateMessage, setUpdateMessage] = useState<string | null>(null);
  const [updatedVersion, setUpdatedVersion] = useState<string | null>(null);

  // Diagnostics
  const [doctorStatus, setDoctorStatus] = useState<"idle" | "running" | "done" | "error">("idle");
  const [doctorOutput, setDoctorOutput] = useState<string | null>(null);

  // Repair
  const [repairStatus, setRepairStatus] = useState<"idle" | "repairing" | "done" | "error">("idle");
  const [repairOutput, setRepairOutput] = useState<string | null>(null);

  // Reinstall confirm dialog (window.confirm doesn't work in macOS WKWebView)
  const [showReinstallConfirm, setShowReinstallConfirm] = useState(false);

  // On mount, re-check gateway status
  const statusCheckDone = useRef(false);
  useEffect(() => {
    if (initialRunning) {
      setChecking(false);
      return;
    }
    if (statusCheckDone.current) return;
    statusCheckDone.current = true;

    (async () => {
      try {
        const status = await invoke<OpenClawStatus>("check_openclaw_status");
        setIsRunning(status.running);
        setChecking(false);
      } catch {
        setChecking(false);
      }
    })();
  }, [initialRunning]);

  // --- Handlers ---

  const handleLaunch = useCallback(async () => {
    setLaunchMessage(null);
    setLaunchStatus("launching");
    try {
      const url = await invoke<string>("launch_openclaw");
      setLaunchStatus("success");
      setLaunchMessage(url);
      setIsRunning(true);
    } catch (err) {
      setLaunchStatus("error");
      setLaunchMessage(String(err));
    }
  }, []);

  const handleOpenWebUI = useCallback(() => {
    openUrl(gatewayUrl);
  }, [gatewayUrl]);

  const handleStop = useCallback(async () => {
    setStopStatus("stopping");
    try {
      await invoke<string>("stop_openclaw_gateway");
      setStopStatus("success");
      setIsRunning(false);
    } catch {
      setStopStatus("error");
    }
  }, []);

  const handleRestart = useCallback(async () => {
    setRestartStatus("restarting");
    try {
      await invoke<string>("restart_openclaw_gateway");
      setRestartStatus("success");
      setIsRunning(true);
    } catch {
      setRestartStatus("error");
    }
  }, []);

  const handleUpdate = useCallback(async () => {
    setUpdateMessage(null);
    setUpdateStatus("updating");
    try {
      const newVersion = await invoke<string>("update_openclaw");
      setUpdateStatus("success");
      setUpdatedVersion(newVersion);
      setUpdateMessage(t("dashboard.updateSuccess", { version: newVersion }));
    } catch (err) {
      setUpdateStatus("error");
      setUpdateMessage(String(err));
    }
  }, [t]);

  const handleDoctor = useCallback(async () => {
    setDoctorOutput(null);
    setDoctorStatus("running");
    try {
      const output = await invoke<string>("openclaw_doctor");
      setDoctorStatus("done");
      setDoctorOutput(output);
    } catch (err) {
      setDoctorStatus("error");
      setDoctorOutput(String(err));
    }
  }, []);

  const handleRepair = useCallback(async () => {
    setRepairOutput(null);
    setRepairStatus("repairing");
    try {
      const output = await invoke<string>("repair_openclaw");
      setRepairStatus("done");
      setRepairOutput(output);
    } catch (err) {
      setRepairStatus("error");
      setRepairOutput(String(err));
    }
  }, []);

  const handleTips = useCallback(() => {
    openUrl(TIPS_URL);
  }, []);

  const displayVersion = updatedVersion ?? currentVersion;
  const showUpdate = needsUpdate && updateStatus !== "success";

  return (
    <div className="dashboard-page">
      <div className="dashboard-scroll">
        {/* Status header */}
        <div className="dashboard-header">
          <div className={`dashboard-icon ${isRunning ? "dashboard-icon--running" : ""}`}>
            <CheckCircle2 />
          </div>
          <h1 className="dashboard-title">{t("dashboard.title")}</h1>
          <p className="dashboard-subtitle">{t("dashboard.subtitle")}</p>
        </div>

        {/* Gateway status */}
        {checking ? (
          <div className="dashboard-status-checking">
            <Loader2 size={16} className="dashboard-spin" />
            <span>{t("dashboard.checkingStatus")}</span>
          </div>
        ) : isRunning ? (
          <div className="dashboard-running-banner" onClick={handleOpenWebUI}>
            <div className="dashboard-running-indicator">
              <Circle size={10} className="dashboard-running-dot" />
              <span>{t("dashboard.gatewayRunning")}</span>
            </div>
            <p className="dashboard-running-hint">{t("dashboard.webUIHint")}</p>
            <button className="dashboard-btn dashboard-btn-webui" onClick={handleOpenWebUI}>
              <Globe size={18} />
              {t("dashboard.openWebUI")}
            </button>
            <span className="dashboard-running-url">{gatewayUrl.split("?")[0]}</span>
          </div>
        ) : (
          <div className="dashboard-stopped-banner">
            <div className="dashboard-stopped-indicator">
              <Circle size={10} className="dashboard-stopped-dot" />
              <span>{t("dashboard.gatewayStopped")}</span>
            </div>
          </div>
        )}

        {/* Version card */}
        <div className="dashboard-version-card">
          <div className="dashboard-version-row">
            <span className="dashboard-version-label">{t("dashboard.version")}</span>
            <span className="dashboard-version-value">v{displayVersion ?? "?"}</span>
          </div>
          {latestVersion && (
            <div className="dashboard-version-row">
              <span className="dashboard-version-label">{t("dashboard.latestVersion")}</span>
              <span className="dashboard-version-value">v{latestVersion}</span>
            </div>
          )}
          <div className="dashboard-version-status">
            {showUpdate ? (
              <span className="dashboard-badge dashboard-badge-warning">
                <ArrowUpCircle size={14} />
                {t("dashboard.updateAvailable")}
              </span>
            ) : (
              <span className="dashboard-badge dashboard-badge-success">
                <CheckCircle2 size={14} />
                {t("dashboard.upToDate")}
              </span>
            )}
          </div>
        </div>

        {/* Update */}
        {showUpdate && (
          <button
            className="dashboard-btn dashboard-btn-update"
            onClick={handleUpdate}
            disabled={updateStatus === "updating"}
          >
            {updateStatus === "updating" ? <Loader2 size={18} className="dashboard-spin" /> : <ArrowUpCircle size={18} />}
            {updateStatus === "updating"
              ? t("dashboard.updating")
              : t("dashboard.updateTo", { version: latestVersion })}
          </button>
        )}
        {updateStatus === "success" && updateMessage && (
          <div className="dashboard-feedback dashboard-feedback-success">
            <CheckCircle2 size={14} />
            <span>{updateMessage}</span>
          </div>
        )}
        {updateStatus === "error" && updateMessage && (
          <div className="dashboard-feedback dashboard-feedback-error">
            <AlertCircle size={14} />
            <span>{updateMessage}</span>
          </div>
        )}

        {/* Gateway management */}
        <h3 className="dashboard-section-title">{t("dashboard.managementTitle")}</h3>
        <div className="dashboard-actions">
          {!isRunning ? (
            <button
              className="dashboard-btn dashboard-btn-primary btn-cta-glow"
              onClick={handleLaunch}
              disabled={launchStatus === "launching"}
            >
              {launchStatus === "launching" ? <Loader2 size={18} className="dashboard-spin" /> : <Rocket size={18} />}
              {launchStatus === "launching" ? t("dashboard.launching") : t("dashboard.launch")}
            </button>
          ) : (
            <button className="dashboard-btn dashboard-btn-primary btn-cta-glow" onClick={handleOpenWebUI}>
              <Globe size={18} />
              {t("dashboard.openWebUI")}
            </button>
          )}

          <div className="dashboard-btn-row">
            <button
              className="dashboard-btn dashboard-btn-secondary dashboard-btn-half"
              onClick={handleRestart}
              disabled={restartStatus === "restarting"}
            >
              {restartStatus === "restarting" ? <Loader2 size={16} className="dashboard-spin" /> : <RefreshCw size={16} />}
              {restartStatus === "restarting" ? t("dashboard.restarting") : t("dashboard.restart")}
            </button>

            <button
              className="dashboard-btn dashboard-btn-secondary dashboard-btn-half"
              onClick={handleStop}
              disabled={stopStatus === "stopping" || !isRunning}
            >
              {stopStatus === "stopping" ? <Loader2 size={16} className="dashboard-spin" /> : <Square size={16} />}
              {stopStatus === "stopping" ? t("dashboard.stopping") : t("dashboard.stopGateway")}
            </button>
          </div>
        </div>

        {/* Launch / restart / stop feedback */}
        {launchStatus === "success" && launchMessage && (
          <div className="dashboard-feedback dashboard-feedback-success">
            <CheckCircle2 size={14} />
            <span>{t("dashboard.launchSuccess")}</span>
          </div>
        )}
        {launchStatus === "error" && launchMessage && (
          <div className="dashboard-feedback dashboard-feedback-error">
            <AlertCircle size={14} />
            <span>{launchMessage}</span>
          </div>
        )}
        {restartStatus === "success" && (
          <div className="dashboard-feedback dashboard-feedback-success">
            <CheckCircle2 size={14} />
            <span>{t("dashboard.restartSuccess")}</span>
          </div>
        )}
        {stopStatus === "success" && (
          <div className="dashboard-feedback dashboard-feedback-success">
            <CheckCircle2 size={14} />
            <span>{t("dashboard.stopSuccess")}</span>
          </div>
        )}

        {/* Diagnostics */}
        <h3 className="dashboard-section-title">{t("dashboard.diagnosticsTitle")}</h3>
        <div className="dashboard-actions">
          <div className="dashboard-btn-row">
            <button
              className="dashboard-btn dashboard-btn-secondary dashboard-btn-half"
              onClick={handleDoctor}
              disabled={doctorStatus === "running"}
            >
              {doctorStatus === "running" ? <Loader2 size={16} className="dashboard-spin" /> : <Stethoscope size={16} />}
              {doctorStatus === "running" ? t("dashboard.doctorRunning") : t("dashboard.doctor")}
            </button>

            <button
              className="dashboard-btn dashboard-btn-secondary dashboard-btn-half"
              onClick={handleRepair}
              disabled={repairStatus === "repairing"}
            >
              {repairStatus === "repairing" ? <Loader2 size={16} className="dashboard-spin" /> : <Wrench size={16} />}
              {repairStatus === "repairing" ? t("dashboard.repairRunning") : t("dashboard.repair")}
            </button>
          </div>
        </div>

        {/* Doctor output */}
        {doctorOutput && (
          <pre className="dashboard-doctor-output">{doctorOutput}</pre>
        )}

        {/* Repair output */}
        {repairOutput && (
          <pre className="dashboard-doctor-output">{repairOutput}</pre>
        )}

        {/* Other actions */}
        <h3 className="dashboard-section-title">{t("dashboard.actions")}</h3>
        <div className="dashboard-actions">
          <button className="dashboard-btn dashboard-btn-secondary" onClick={onReconfigureApi}>
            <Settings size={16} />
            {t("dashboard.reconfigureApi")}
          </button>

          <button className="dashboard-btn dashboard-btn-secondary" onClick={onConfigureFeishu}>
            <MessageSquare size={16} />
            {t("dashboard.configureFeishu")}
          </button>

          <button className="dashboard-btn dashboard-btn-secondary" onClick={handleTips}>
            <Sparkles size={16} />
            {t("dashboard.viewTutorial")}
          </button>

          <button
            className="dashboard-btn dashboard-btn-danger"
            onClick={() => setShowReinstallConfirm(true)}
          >
            <RotateCcw size={16} />
            {t("dashboard.reinstall")}
          </button>
        </div>
      </div>

      {/* Reinstall confirm dialog */}
      {showReinstallConfirm && (
        <div className="dashboard-confirm-overlay" onClick={() => setShowReinstallConfirm(false)}>
          <div className="dashboard-confirm-dialog" onClick={(e) => e.stopPropagation()}>
            <AlertCircle size={24} className="dashboard-confirm-icon" />
            <p className="dashboard-confirm-text">{t("dashboard.reinstallConfirm")}</p>
            <div className="dashboard-confirm-actions">
              <button
                className="dashboard-btn dashboard-btn-secondary"
                onClick={() => setShowReinstallConfirm(false)}
              >
                {t("common.cancel")}
              </button>
              <button
                className="dashboard-btn dashboard-btn-danger"
                onClick={() => {
                  setShowReinstallConfirm(false);
                  onReinstall();
                }}
              >
                {t("dashboard.reinstall")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
