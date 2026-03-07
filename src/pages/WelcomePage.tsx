import { useTranslation } from "react-i18next";
import { useStepNavigation } from "../hooks/useStepNavigation";
import { Shell, ArrowRight } from "lucide-react";
import "./WelcomePage.css";

const APP_VERSION = "0.1.0";

export default function WelcomePage() {
  const { t } = useTranslation();
  const { goToStep } = useStepNavigation();

  const handleStart = () => {
    goToStep(1);
  };

  return (
    <div className="welcome-page">
      {/* Background - matches website hero */}
      <div className="welcome-bg">
        <div className="welcome-glow welcome-glow-1" />
        <div className="welcome-glow welcome-glow-2" />
        <div className="welcome-grid" />
      </div>

      {/* Logo */}
      <div className="welcome-logo">
        <Shell />
      </div>

      {/* Title - matches website hero-title style */}
      <h1 className="welcome-title">
        <span className="welcome-title-icon">🦞</span>
        <span>
          <span className="text-mala">麻辣</span>
          <span className="text-longxia">龙虾</span>
        </span>
      </h1>
      <p className="welcome-subtitle">{t("welcome.subtitle")}</p>

      {/* Start button */}
      <button className="welcome-start-btn btn-cta-glow" onClick={handleStart}>
        <span>{t("btn.start")}</span>
        <ArrowRight size={20} />
      </button>

      {/* Version */}
      <span className="welcome-version">v{APP_VERSION}</span>
    </div>
  );
}
