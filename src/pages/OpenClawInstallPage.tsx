import { useState, useCallback, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useInstallStore } from "../stores/useInstallStore";
import { useStepNavigation } from "../hooks/useStepNavigation";
import type { Mirror } from "../types";
import { Info } from "lucide-react";
import clsx from "clsx";
import "./OpenClawInstallPage.css";

// npm registry mirror sources for OpenClaw installation
const NPM_MIRRORS: readonly Mirror[] = [
  { name: "mirror.npmmirror", url: "https://registry.npmmirror.com", type: "npm" },
  { name: "mirror.tencent", url: "https://mirrors.cloud.tencent.com/npm/", type: "npm" },
  { name: "mirror.huawei", url: "https://repo.huaweicloud.com/repository/npm/", type: "npm" },
  { name: "mirror.official", url: "https://registry.npmjs.org", type: "npm" },
];

// Default mirror (npmmirror / taobao)
const DEFAULT_MIRROR = NPM_MIRRORS[0];

export default function OpenClawInstallPage() {
  const { t } = useTranslation();
  const {
    openclawInstallStatus,
    openclawInstallLogs,
    selectedMirror,
    setSelectedMirror,
    setOpenclawInstallStatus,
    setOpenclawVersion,
    addOpenclawInstallLog,
  } = useInstallStore();
  const { goToStep } = useStepNavigation();

  const [mirrorLatencies, setMirrorLatencies] = useState<Record<string, number | null>>({});
  const [isTesting, setIsTesting] = useState(false);
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
      addOpenclawInstallLog({
        timestamp: Date.now(),
        level: "info",
        message: event.payload,
      });
    }).then((unlisten) => unlisteners.push(unlisten));

    return () => {
      unlisteners.forEach((fn) => fn());
    };
  }, [addOpenclawInstallLog]);

  // Test latency for all mirrors in parallel
  const testMirrorSpeed = useCallback(async () => {
    setIsTesting(true);

    const entries = await Promise.allSettled(
      NPM_MIRRORS.map(async (mirror) => {
        try {
          const latency = await invoke<number>("test_mirror_latency", { url: mirror.url });
          return { url: mirror.url, latency };
        } catch {
          return { url: mirror.url, latency: null as number | null };
        }
      })
    );

    const results: Record<string, number | null> = {};
    for (const entry of entries) {
      if (entry.status === "fulfilled") {
        results[entry.value.url] = entry.value.latency;
      }
    }

    setMirrorLatencies(results);

    // Auto-select fastest available mirror
    const fastest = Object.entries(results)
      .filter(([, lat]) => lat != null)
      .sort(([, a], [, b]) => (a ?? Infinity) - (b ?? Infinity))[0];

    if (fastest) {
      const fastestMirror = NPM_MIRRORS.find((m) => m.url === fastest[0]) ?? null;
      setSelectedMirror(fastestMirror);
    }

    setIsTesting(false);
  }, [setSelectedMirror]);

  // Auto-test speed on mount, default select npmmirror
  useEffect(() => {
    if (!selectedMirror) {
      setSelectedMirror(DEFAULT_MIRROR);
    }
    testMirrorSpeed();
    // Only run once on mount
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Start OpenClaw installation
  const handleInstall = useCallback(async () => {
    if (!selectedMirror) return;

    setOpenclawInstallStatus("installing");
    addOpenclawInstallLog({
      timestamp: Date.now(),
      level: "info",
      message: t("openclawInstall.installing"),
    });

    try {
      const result = await invoke<{ version: string }>("install_openclaw", {
        mirror: selectedMirror.url,
      });
      setOpenclawVersion(result.version);
      setOpenclawInstallStatus("success");
      addOpenclawInstallLog({
        timestamp: Date.now(),
        level: "info",
        message: t("openclawInstall.complete"),
      });
    } catch (err) {
      setOpenclawInstallStatus("error");
      addOpenclawInstallLog({
        timestamp: Date.now(),
        level: "error",
        message: String(err),
      });
    }
  }, [
    selectedMirror,
    setOpenclawInstallStatus,
    setOpenclawVersion,
    addOpenclawInstallLog,
    t,
  ]);

  const isInstalling = openclawInstallStatus === "installing";
  const isComplete = openclawInstallStatus === "success";
  const isFailed = openclawInstallStatus === "error";

  // Find the fastest mirror url
  const fastestUrl = Object.entries(mirrorLatencies)
    .filter(([, lat]) => lat != null)
    .sort(([, a], [, b]) => (a ?? Infinity) - (b ?? Infinity))[0]?.[0];

  // "下一步" triggers install if idle, navigates if complete
  const handleNext = useCallback(async () => {
    if (isComplete) {
      goToStep(4);
      return;
    }
    // Allow retry after error
    if ((openclawInstallStatus === "idle" || openclawInstallStatus === "error") && selectedMirror) {
      setOpenclawInstallStatus("idle");
      await handleInstall();
    }
  }, [isComplete, openclawInstallStatus, selectedMirror, handleInstall, goToStep, setOpenclawInstallStatus]);

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
  }, [openclawInstallLogs]);

  // Auto-navigate on success
  useEffect(() => {
    if (isComplete) {
      goToStep(4);
    }
  }, [isComplete, goToStep]);

  const handleBack = () => {
    goToStep(2);
  };

  return (
    <div className="ocinstall-page">
      <div className="ocinstall-header">
        <h1>{t("openclawInstall.title")}</h1>
        <p>{t("openclawInstall.description")}</p>
      </div>

      <div className="ocinstall-content">
        {/* Mirror strategy explanation */}
        <div className="ocinstall-strategy">
          <Info />
          <span className="ocinstall-strategy-text">
            {t("openclawInstall.strategy")}
          </span>
        </div>

        {/* Mirror list with speed test */}
        <div className="ocinstall-mirrors">
          <h3>
            {t("openclawInstall.selectMirror")}
            <button
              className="ocinstall-speedtest-btn"
              onClick={testMirrorSpeed}
              disabled={isTesting || isInstalling}
            >
              {isTesting ? t("mirror.testing") : t("mirror.testSpeed")}
            </button>
          </h3>

          <div className="ocinstall-mirror-list">
            {NPM_MIRRORS.map((mirror) => {
              const latency = mirrorLatencies[mirror.url];
              const isFastest = mirror.url === fastestUrl && fastestUrl != null;
              return (
                <div
                  key={mirror.url}
                  className={clsx(
                    "ocinstall-mirror",
                    selectedMirror?.url === mirror.url && "selected",
                    isFastest && "fastest"
                  )}
                  onClick={() => !isInstalling && setSelectedMirror(mirror)}
                >
                  <div className="ocinstall-mirror-radio" />
                  <div className="ocinstall-mirror-info">
                    <div className="ocinstall-mirror-name">{t(mirror.name)}</div>
                    <div className="ocinstall-mirror-url">{mirror.url}</div>
                  </div>
                  {isFastest && (
                    <span className="ocinstall-mirror-badge">
                      {t("openclawInstall.fastest")}
                    </span>
                  )}
                  <span className="ocinstall-mirror-latency">
                    {latency != null
                      ? `${latency}ms`
                      : latency === null && Object.keys(mirrorLatencies).length > 0
                        ? t("openclawInstall.timeout")
                        : t("mirror.untested")}
                  </span>
                </div>
              );
            })}
          </div>
        </div>

        {/* Progress */}
        {isInstalling && (
          <div ref={progressRef} className="ocinstall-progress">
            <div className="ocinstall-progress-status">
              {installMessage || t("openclawInstall.installing")}
            </div>
            <div className="ocinstall-progress-bar-container">
              {installPercent > 0 ? (
                <div
                  className="ocinstall-progress-bar"
                  style={{ width: `${installPercent}%` }}
                />
              ) : (
                <div className="ocinstall-progress-bar indeterminate" />
              )}
            </div>
            {installPercent > 0 && (
              <div className="ocinstall-progress-percent">{installPercent}%</div>
            )}
          </div>
        )}

        {/* Log viewer */}
        {openclawInstallLogs.length > 0 && (
          <div ref={logsRef} className="ocinstall-logs">
            {openclawInstallLogs.map((log, i) => (
              <div key={i} className={clsx("ocinstall-log-entry", log.level)}>
                {log.message}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Navigation */}
      <div className="ocinstall-actions">
        <button
          className="ocinstall-btn ocinstall-btn-secondary"
          onClick={handleBack}
          disabled={isInstalling}
        >
          {t("btn.prev")}
        </button>
        <button
          className={clsx(
            "ocinstall-btn ocinstall-btn-primary",
            !isInstalling && selectedMirror && "btn-cta-glow"
          )}
          disabled={isInstalling || (!isComplete && !isFailed && !selectedMirror)}
          onClick={handleNext}
        >
          {isInstalling
            ? t("openclawInstall.installing")
            : t("btn.next")}
        </button>
      </div>
    </div>
  );
}
