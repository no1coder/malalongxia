import { useState, useCallback, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useInstallStore } from "../stores/useInstallStore";
import { useStepNavigation } from "../hooks/useStepNavigation";
import { useMirrorConfig } from "../hooks/useMirrorConfig";
import type { NodeVerifyResult } from "../types";
import { Info, AlertTriangle, Loader2 } from "lucide-react";
import clsx from "clsx";
import "./OpenClawInstallPage.css";

export default function OpenClawInstallPage() {
  const { t } = useTranslation();
  const {
    openclawInstallStatus,
    openclawInstallLogs,
    setOpenclawInstallStatus,
    setOpenclawVersion,
    addOpenclawInstallLog,
    osType,
  } = useInstallStore();
  const { goToStep } = useStepNavigation();
  const { npmMirrors, isLoading: mirrorsLoading } = useMirrorConfig();

  // Local npm mirror selection (independent from Node mirror in previous step)
  const [selectedNpmMirror, setSelectedNpmMirror] = useState<import("../types").Mirror | null>(null);
  const [mirrorLatencies, setMirrorLatencies] = useState<Record<string, number | null>>({});
  const [isTesting, setIsTesting] = useState(false);
  const [installPercent, setInstallPercent] = useState(0);
  const [installMessage, setInstallMessage] = useState("");
  const [npmReady, setNpmReady] = useState<boolean | null>(null); // null = checking
  const progressRef = useRef<HTMLDivElement>(null);
  const logsRef = useRef<HTMLDivElement>(null);

  // Listen for install progress and log events from backend
  useEffect(() => {
    const unlisteners: (() => void)[] = [];

    listen<{ percent: number; message: string }>("openclaw-install-progress", (event) => {
      setInstallPercent(event.payload.percent);
      setInstallMessage(event.payload.message);
    }).then((unlisten) => unlisteners.push(unlisten));

    listen<string>("openclaw-install-log", (event) => {
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
    if (npmMirrors.length === 0) return;
    setIsTesting(true);

    const entries = await Promise.allSettled(
      npmMirrors.map(async (mirror) => {
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
      const fastestMirror = npmMirrors.find((m) => m.url === fastest[0]) ?? null;
      setSelectedNpmMirror(fastestMirror);
    }

    setIsTesting(false);
  }, [npmMirrors, setSelectedNpmMirror]);

  // Pre-flight: verify npm is available before allowing install
  useEffect(() => {
    invoke<NodeVerifyResult>("verify_node_npm")
      .then((result) => {
        setNpmReady(result.npm_available);
      })
      .catch(() => {
        setNpmReady(false);
      });
  }, []);

  // Auto-test speed once mirrors are loaded, default select first mirror
  useEffect(() => {
    if (mirrorsLoading || npmMirrors.length === 0) return;
    if (!selectedNpmMirror) {
      setSelectedNpmMirror(npmMirrors[0]);
    }
    testMirrorSpeed();
    // Only run when mirrors become available
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mirrorsLoading]);

  // Start OpenClaw installation
  const handleInstall = useCallback(async () => {
    if (!selectedNpmMirror) return;

    setOpenclawInstallStatus("installing");
    addOpenclawInstallLog({
      timestamp: Date.now(),
      level: "info",
      message: t("openclawInstall.installing"),
    });

    try {
      const result = await invoke<{ version: string }>("install_openclaw", {
        mirror: selectedNpmMirror.url,
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
    selectedNpmMirror,
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

  // "下一步" only navigates when install is complete
  const handleNext = useCallback(() => {
    if (isComplete) {
      goToStep(4);
    }
  }, [isComplete, goToStep]);

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

  const handleBack = () => {
    goToStep(2);
  };

  return (
    <div className="ocinstall-page">
      <div className="ocinstall-header">
        <h1>{t("openclawInstall.title")}</h1>
        <p>{t("openclawInstall.description")}</p>
      </div>

      {/* Sticky top notices */}
      <div className="ocinstall-notices">
        {osType === "windows" && (
          <div className="ocinstall-notice ocinstall-notice-warn">
            <AlertTriangle size={16} />
            <span>{t("openclawInstall.winConsoleWarning")}</span>
          </div>
        )}
        <div className="ocinstall-notice ocinstall-notice-info">
          <Info size={16} />
          <span>{t("openclawInstall.strategy")}</span>
        </div>
      </div>

      <div className="ocinstall-content">
        {/* npm not available warning */}
        {npmReady === false && (
          <div className="ocinstall-npm-error">
            <AlertTriangle />
            <div>
              <div className="ocinstall-npm-error-title">{t("openclawInstall.npmNotAvailable")}</div>
              <div className="ocinstall-npm-error-hint">{t("openclawInstall.npmNotAvailableHint")}</div>
            </div>
          </div>
        )}

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

          {mirrorsLoading ? (
            <div className="ocinstall-mirrors-loading">
              <Loader2 className="spin" size={16} />
              <span>{t("mirror.loadingConfig")}</span>
            </div>
          ) : (
            <div className="ocinstall-mirror-list">
              {npmMirrors.map((mirror) => {
                const latency = mirrorLatencies[mirror.url];
                const isFastest = mirror.url === fastestUrl && fastestUrl != null;
                return (
                  <div
                    key={mirror.url}
                    className={clsx(
                      "ocinstall-mirror",
                      selectedNpmMirror?.url === mirror.url && "selected",
                      isFastest && "fastest"
                    )}
                    onClick={() => !isInstalling && setSelectedNpmMirror(mirror)}
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
          )}
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

        {/* Install / Retry button — only before success */}
        {!isComplete && (
          <button
            className={clsx(
              "ocinstall-btn ocinstall-btn-primary",
              !isInstalling && selectedNpmMirror && npmReady === true && "btn-cta-glow"
            )}
            disabled={isInstalling || npmReady !== true || !selectedNpmMirror}
            onClick={handleInstall}
          >
            {isInstalling
              ? t("openclawInstall.installing")
              : isFailed
                ? t("btn.retry")
                : t("openclawInstall.installBtn")}
          </button>
        )}

        {/* Next button — enabled only after success */}
        <button
          className={clsx(
            "ocinstall-btn ocinstall-btn-primary",
            isComplete && "btn-cta-glow"
          )}
          disabled={!isComplete}
          onClick={handleNext}
        >
          {t("btn.next")}
        </button>
      </div>
    </div>
  );
}
