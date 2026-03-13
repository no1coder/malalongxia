import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useInstallStore } from "../stores/useInstallStore";
import {
  CheckCircle2,
  Rocket,
  Sparkles,
  Wrench,
  Users,
  Bell,
  AlertCircle,
} from "lucide-react";
import qrCodeFallback from "../assets/qrcode.png";
import "./CompletionPage.css";

const TIPS_URL = "https://malalongxia.com/tips.html";
const QR_URL = "https://malalongxia.com/qrcode.png";

const BENEFITS = [
  { icon: Sparkles, color: "rgba(96, 165, 250, 0.15)", stroke: "#60A5FA", titleKey: "completion.benefitSkill", descKey: "completion.benefitSkillDesc" },
  { icon: Wrench, color: "rgba(52, 211, 153, 0.15)", stroke: "#34D399", titleKey: "completion.benefitSupport", descKey: "completion.benefitSupportDesc" },
  { icon: Users, color: "rgba(251, 191, 36, 0.15)", stroke: "#FBBF24", titleKey: "completion.benefitCommunity", descKey: "completion.benefitCommunityDesc" },
  { icon: Bell, color: "rgba(255, 77, 77, 0.15)", stroke: "#ff4d4d", titleKey: "completion.benefitBeta", descKey: "completion.benefitBetaDesc" },
] as const;

interface CompletionPageProps {
  readonly onComplete?: () => void;
}

export default function CompletionPage({ onComplete }: CompletionPageProps) {
  const { t } = useTranslation();
  const { nodeVersion, nodeRequired, openclawVersion, selectedProvider } = useInstallStore();
  const [launchStatus, setLaunchStatus] = useState<"idle" | "launching" | "success" | "error">("idle");
  const [launchMessage, setLaunchMessage] = useState<string | null>(null);
  const redirectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Clean up redirect timer on unmount
  useEffect(() => {
    return () => {
      if (redirectTimerRef.current !== null) {
        clearTimeout(redirectTimerRef.current);
      }
    };
  }, []);

  // Launch OpenClaw via Tauri backend, then transition to dashboard
  const handleLaunch = useCallback(async () => {
    setLaunchMessage(null);
    setLaunchStatus("launching");
    try {
      const url = await invoke<string>("launch_openclaw");
      setLaunchStatus("success");
      setLaunchMessage(url);
      // Transition to dashboard after a brief delay so user sees the success state
      if (onComplete) {
        redirectTimerRef.current = setTimeout(onComplete, 1500);
      }
    } catch (err) {
      setLaunchStatus("error");
      // Show only the message part if it's a Tauri error string like "xxx: message"
      const raw = String(err);
      const colonIdx = raw.indexOf(": ");
      setLaunchMessage(colonIdx !== -1 ? raw.slice(colonIdx + 2) : raw);
    }
  }, [onComplete]);

  const handleTips = useCallback(() => {
    openUrl(TIPS_URL);
  }, []);

  return (
    <div className="completion-page">
      <div className="completion-scroll">
        {/* Success icon */}
        <div className="completion-icon">
          <CheckCircle2 />
        </div>

        <h1 className="completion-title">
          {t("completion.congratulations")}
        </h1>
        <p className="completion-desc">{t("completion.openclawReady")}</p>

        {/* Installation summary */}
        <div className="completion-summary">
          <h3 className="completion-summary-title">
            {t("completion.summaryTitle")}
          </h3>
          <div className="completion-summary-list">
            <div className="completion-summary-item">
              <span className="completion-summary-label">Node.js</span>
              <span className="completion-summary-value">
                {nodeVersion
                  ? nodeVersion
                  : nodeRequired
                    ? t("completion.notInstalled")
                    : t("envCheck.statusPass")}
              </span>
            </div>
            <div className="completion-summary-item">
              <span className="completion-summary-label">OpenClaw</span>
              <span className="completion-summary-value">
                {openclawVersion ?? t("completion.notInstalled")}
              </span>
            </div>
            <div className="completion-summary-item">
              <span className="completion-summary-label">
                {t("apiConfig.selectProvider")}
              </span>
              <span className="completion-summary-value">
                {selectedProvider
                  ? t(selectedProvider.name)
                  : t("completion.notConfigured")}
              </span>
            </div>
          </div>
        </div>

        {/* Benefits grid (tips-style) */}
        <div className="completion-benefits">
          {BENEFITS.map((b) => {
            const Icon = b.icon;
            return (
              <div key={b.titleKey} className="completion-benefit">
                <div
                  className="completion-benefit-icon"
                  style={{ background: b.color }}
                >
                  <Icon size={20} color={b.stroke} />
                </div>
                <div>
                  <div className="completion-benefit-title">{t(b.titleKey)}</div>
                  <div className="completion-benefit-desc">{t(b.descKey)}</div>
                </div>
              </div>
            );
          })}
        </div>

        {/* QR code CTA */}
        <div className="completion-qr-cta">
          <h3>{t("completion.scanTitle")}</h3>
          <p>{t("completion.scanDesc")}</p>
          <div className="completion-qr-wrap">
            <img
              src={QR_URL}
              alt="WeChat QR"
              width="180"
              height="180"
              onError={(e) => { e.currentTarget.src = qrCodeFallback; }}
            />
          </div>
          <p className="completion-qr-hint">{t("completion.scanHint")}</p>
        </div>

        {/* Action buttons */}
        <div className="completion-buttons">
          <button
            className="completion-btn completion-btn-primary btn-cta-glow"
            onClick={handleLaunch}
            disabled={launchStatus === "launching" || launchStatus === "success"}
          >
            <Rocket size={18} />
            {launchStatus === "launching"
              ? t("completion.launching")
              : t("completion.startUsing")}
          </button>
          {launchStatus === "success" && launchMessage && (
            <div className="completion-launch-success">
              <CheckCircle2 size={14} />
              <span>{t("completion.launchSuccess", { url: launchMessage })}</span>
            </div>
          )}
          {launchStatus === "error" && launchMessage && (
            <div className="completion-launch-error">
              <AlertCircle size={14} />
              <span>{launchMessage}</span>
            </div>
          )}
          {launchStatus === "error" && onComplete && (
            <button className="completion-btn completion-btn-secondary" onClick={onComplete}>
              {t("completion.skipToDashboard")}
            </button>
          )}
          <button className="completion-btn completion-btn-secondary" onClick={handleTips}>
            <Sparkles size={18} />
            {t("completion.viewTutorial")}
          </button>
        </div>
      </div>
    </div>
  );
}
