import { type ReactNode, useState } from "react";
import { useTranslation } from "react-i18next";
import { Languages, CircleHelp, ExternalLink } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import appIcon from "../assets/app-icon.png";
import { StepIndicator, type StepInfo } from "./StepIndicator";

interface LayoutProps {
  readonly children: ReactNode;
  readonly currentStep: number;
  readonly steps: readonly StepInfo[];
  readonly banner?: ReactNode;
}

// Main layout: header on top, sidebar (steps) on the left, content on the right
export function Layout({
  children,
  currentStep,
  steps,
  banner,
}: LayoutProps) {
  const { t, i18n } = useTranslation();

  const toggleLanguage = () => {
    const nextLang = i18n.language === "zh-CN" ? "en-US" : "zh-CN";
    i18n.changeLanguage(nextLang);
    try {
      localStorage.setItem("app-language", nextLang);
    } catch {
      // localStorage may be unavailable
    }
  };

  const isZh = i18n.language === "zh-CN";
  const [showQr, setShowQr] = useState(false);

  return (
    <div className="layout">
      {/* Header */}
      <header className="layout__header">
        <div
          className="layout__brand layout__brand--link"
          onClick={() => openUrl("https://malalongxia.com")}
          role="link"
          tabIndex={0}
          onKeyDown={(e) => { if (e.key === "Enter") openUrl("https://malalongxia.com"); }}
        >
          <img className="layout__logo" src={appIcon} alt="OpenClawX" />
          <h1 className="layout__title">OpenClawX</h1>
        </div>

        <div className="layout__header-actions">
          <button
            type="button"
            className="layout__action-btn"
            onClick={() => openUrl("https://malalongxia.com")}
            title={t("header.website")}
          >
            <ExternalLink size={18} />
            <span>{t("header.website")}</span>
          </button>
          <button
            type="button"
            className="layout__action-btn"
            onClick={toggleLanguage}
            title="Switch language"
          >
            <Languages size={18} />
            <span>{isZh ? t("lang.en") : t("lang.zh")}</span>
          </button>
          <button
            type="button"
            className="layout__action-btn"
            onClick={() => setShowQr(true)}
            title={t("completion.contactAuthor")}
          >
            <CircleHelp size={18} />
          </button>
        </div>
      </header>

      {/* WeChat QR modal */}
      {showQr && (
        <div className="layout__qr-overlay" onClick={() => setShowQr(false)}>
          <div className="layout__qr-modal" onClick={(e) => e.stopPropagation()}>
            <h3>{t("completion.contactTitle")}</h3>
            <p>{t("completion.contactDesc")}</p>
            <div className="layout__qr-img">
              <img src="https://malalongxia.com/qrcode.png" alt="WeChat QR" width="200" height="200" />
            </div>
            <button className="layout__qr-close" onClick={() => setShowQr(false)}>
              {t("common.close")}
            </button>
          </div>
        </div>
      )}

      {/* Update banner */}
      {banner}

      {/* Body: sidebar + content */}
      <div className="layout__body">
        {/* Left sidebar with step navigation (hidden in dashboard mode) */}
        {steps.length > 0 && (
          <aside className="layout__sidebar">
            <StepIndicator
              currentStep={currentStep}
              totalSteps={steps.length}
              steps={steps}
            />
          </aside>
        )}

        {/* Right content area */}
        <main className="layout__content">{children}</main>
      </div>
    </div>
  );
}
