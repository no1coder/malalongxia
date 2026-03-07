import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  CheckCircle2,
  XCircle,
  Loader2,
  ExternalLink,
  Download,
  MessageSquare,
} from "lucide-react";
import "./FeishuConfigPage.css";

const FEISHU_DOCS_URL = "https://docs.openclaw.ai/channels/feishu";

interface FeishuConfigPageProps {
  readonly onDone: () => void;
}

export default function FeishuConfigPage({ onDone }: FeishuConfigPageProps) {
  const { t } = useTranslation();

  // Plugin install state
  const [pluginInstalled, setPluginInstalled] = useState<boolean | null>(null);
  const [installStatus, setInstallStatus] = useState<"idle" | "installing" | "success" | "error">("idle");
  const [installError, setInstallError] = useState<string | null>(null);

  // Config form state
  const [appId, setAppId] = useState("");
  const [appSecret, setAppSecret] = useState("");
  const [configStatus, setConfigStatus] = useState<"idle" | "saving" | "success" | "error">("idle");
  const [configError, setConfigError] = useState<string | null>(null);

  // Check if plugin is installed on mount
  const checkDone = useRef(false);
  useEffect(() => {
    if (checkDone.current) return;
    checkDone.current = true;

    (async () => {
      try {
        const installed = await invoke<boolean>("check_feishu_plugin");
        setPluginInstalled(installed);
      } catch {
        setPluginInstalled(false);
      }
    })();
  }, []);

  const handleInstallPlugin = useCallback(async () => {
    setInstallStatus("installing");
    setInstallError(null);
    try {
      await invoke<string>("install_feishu_plugin");
      setInstallStatus("success");
      setPluginInstalled(true);
    } catch (err) {
      setInstallStatus("error");
      setInstallError(String(err));
    }
  }, []);

  const handleSave = useCallback(async () => {
    if (!appId.trim() || !appSecret.trim()) return;

    setConfigStatus("saving");
    setConfigError(null);
    try {
      await invoke("configure_feishu", {
        appId: appId.trim(),
        appSecret: appSecret.trim(),
      });
      setConfigStatus("success");
    } catch (err) {
      setConfigStatus("error");
      setConfigError(String(err));
    }
  }, [appId, appSecret]);

  const handleOpenDocs = useCallback(() => {
    openUrl(FEISHU_DOCS_URL);
  }, []);

  const isInstalling = installStatus === "installing";
  const isSaving = configStatus === "saving";

  return (
    <div className="feishu-page">
      <div className="feishu-scroll">
        {/* Header */}
        <div className="feishu-header">
          <div className="feishu-icon">
            <MessageSquare />
          </div>
          <h1>{t("feishu.title")}</h1>
          <p>{t("feishu.description")}</p>
        </div>

        {/* Docs link */}
        <button className="feishu-docs-btn" onClick={handleOpenDocs}>
          <ExternalLink size={16} />
          {t("feishu.viewDocs")}
        </button>

        {/* Step 1: Install plugin */}
        <div className="feishu-section">
          <h3 className="feishu-section-title">{t("feishu.step1")}</h3>

          {pluginInstalled === null ? (
            <div className="feishu-status">
              <Loader2 size={16} className="feishu-spin" />
              <span>{t("common.loading")}</span>
            </div>
          ) : pluginInstalled ? (
            <div className="feishu-status feishu-status-success">
              <CheckCircle2 size={16} />
              <span>{t("feishu.pluginInstalled")}</span>
            </div>
          ) : (
            <>
              <button
                className="feishu-install-btn"
                onClick={handleInstallPlugin}
                disabled={isInstalling}
              >
                {isInstalling ? (
                  <Loader2 size={16} className="feishu-spin" />
                ) : (
                  <Download size={16} />
                )}
                {isInstalling ? t("feishu.installing") : t("feishu.installPlugin")}
              </button>
              {installStatus === "error" && installError && (
                <div className="feishu-feedback feishu-feedback-error">
                  <XCircle size={14} />
                  <span>{installError}</span>
                </div>
              )}
            </>
          )}

          {installStatus === "success" && (
            <div className="feishu-feedback feishu-feedback-success">
              <CheckCircle2 size={14} />
              <span>{t("feishu.installSuccess")}</span>
            </div>
          )}
        </div>

        {/* Step 2: Configure credentials */}
        {pluginInstalled && (
          <div className="feishu-section">
            <h3 className="feishu-section-title">{t("feishu.step2")}</h3>

            <div className="feishu-field">
              <label className="feishu-label">App ID</label>
              <input
                className="feishu-input"
                type="text"
                placeholder={t("feishu.appIdPlaceholder")}
                value={appId}
                onChange={(e) => setAppId(e.target.value)}
              />
            </div>

            <div className="feishu-field">
              <label className="feishu-label">App Secret</label>
              <input
                className="feishu-input"
                type="password"
                placeholder={t("feishu.appSecretPlaceholder")}
                value={appSecret}
                onChange={(e) => setAppSecret(e.target.value)}
              />
            </div>

            <button
              className="feishu-save-btn"
              onClick={handleSave}
              disabled={!appId.trim() || !appSecret.trim() || isSaving}
            >
              {isSaving ? (
                <Loader2 size={16} className="feishu-spin" />
              ) : null}
              {isSaving ? t("common.loading") : t("feishu.saveConfig")}
            </button>

            {configStatus === "success" && (
              <div className="feishu-feedback feishu-feedback-success">
                <CheckCircle2 size={14} />
                <span>{t("feishu.configSuccess")}</span>
              </div>
            )}
            {configStatus === "error" && configError && (
              <div className="feishu-feedback feishu-feedback-error">
                <XCircle size={14} />
                <span>{configError}</span>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Bottom action */}
      <div className="feishu-actions">
        <button className="feishu-btn-return" onClick={onDone}>
          {t("apiConfig.saveAndReturn")}
        </button>
      </div>
    </div>
  );
}
