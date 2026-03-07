import { useCallback, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useInstallStore } from "../stores/useInstallStore";
import { useStepNavigation } from "../hooks/useStepNavigation";
import type { AIProviderOption } from "../stores/useInstallStore";
import {
  Bot,
  Brain,
  Moon,
  Sparkles,
  Cloud,
  Globe,
  Shield,
  Settings,
  CheckCircle2,
  XCircle,
  Loader2,
  ExternalLink,
} from "lucide-react";
import clsx from "clsx";
import "./ApiConfigPage.css";

// Provider console URLs for obtaining API keys
const PROVIDER_KEY_URLS: Record<string, string> = {
  zhipu: "https://open.bigmodel.cn/usercenter/apikeys",
  qwen: "https://bailian.console.aliyun.com/cn-beijing/?tab=model#/efm/coding_plan",
  moonshot: "https://platform.moonshot.cn/console/api-keys",
  deepseek: "https://platform.deepseek.com/api_keys",
  wenxin: "https://console.bce.baidu.com/qianfan/ais/console/applicationConsole/application",
  openai: "https://platform.openai.com/api-keys",
  anthropic: "https://console.anthropic.com/settings/keys",
};

// Available AI provider options
const AI_PROVIDERS: readonly AIProviderOption[] = [
  {
    id: "zhipu",
    name: "apiConfig.zhipu",
    description: "apiConfig.zhipuDesc",
    baseUrl: "https://open.bigmodel.cn/api/paas/v4",
    needsProxy: false,
    openclawProvider: "zai",
    defaultModel: "glm-5",
  },
  {
    id: "qwen",
    name: "apiConfig.qwen",
    description: "apiConfig.qwenDesc",
    baseUrl: "https://coding.dashscope.aliyuncs.com/v1",
    needsProxy: false,
    recommended: true,
    openclawProvider: "bailian",
    defaultModel: "qwen3.5-plus",
  },
  {
    id: "moonshot",
    name: "apiConfig.moonshot",
    description: "apiConfig.moonshotDesc",
    baseUrl: "https://api.moonshot.cn/v1",
    needsProxy: false,
    openclawProvider: "moonshot",
    defaultModel: "kimi-k2.5",
  },
  {
    id: "deepseek",
    name: "apiConfig.deepseek",
    description: "apiConfig.deepseekDesc",
    baseUrl: "https://api.deepseek.com/v1",
    needsProxy: false,
    openclawProvider: "deepseek",
    defaultModel: "deepseek-chat",
  },
  {
    id: "wenxin",
    name: "apiConfig.wenxin",
    description: "apiConfig.wenxinDesc",
    baseUrl: "https://aip.baidubce.com/rpc/2.0/ai_custom/v1",
    needsProxy: false,
    openclawProvider: "qianfan",
    defaultModel: "ernie-4.5",
  },
  {
    id: "openai",
    name: "apiConfig.openai",
    description: "apiConfig.openaiDesc",
    baseUrl: "https://api.openai.com/v1",
    needsProxy: true,
    openclawProvider: "openai",
    defaultModel: "gpt-4o",
  },
  {
    id: "anthropic",
    name: "apiConfig.anthropic",
    description: "apiConfig.anthropicDesc",
    baseUrl: "https://api.anthropic.com/v1",
    needsProxy: true,
    openclawProvider: "anthropic",
    defaultModel: "claude-sonnet-4-5",
  },
  {
    id: "custom",
    name: "apiConfig.custom",
    description: "apiConfig.customDesc",
    baseUrl: "",
    needsProxy: false,
    openclawProvider: "custom",
    defaultModel: "",
  },
];

// Map provider id to icon component
const PROVIDER_ICONS: Record<string, React.ElementType> = {
  zhipu: Brain,
  qwen: Sparkles,
  moonshot: Moon,
  deepseek: Bot,
  wenxin: Cloud,
  openai: Globe,
  anthropic: Shield,
  custom: Settings,
};

interface ApiConfigPageProps {
  readonly mode?: "wizard" | "reconfig";
  readonly onDone?: () => void;
}

export default function ApiConfigPage({ mode = "wizard", onDone }: ApiConfigPageProps) {
  const { t } = useTranslation();
  const {
    selectedProvider,
    apiKey,
    apiBaseUrl,
    apiModel,
    apiTestStatus,
    setSelectedProvider,
    setApiKey,
    setApiBaseUrl,
    setApiModel,
    setApiTestStatus,
  } = useInstallStore();
  const { goToStep } = useStepNavigation();

  const isReconfig = mode === "reconfig";
  const formRef = useRef<HTMLDivElement>(null);
  const [testError, setTestError] = useState<string | null>(null);

  // Test API connection via OpenAI-compatible request
  const handleTestConnection = useCallback(async () => {
    if (!selectedProvider || !apiKey) return;

    setApiTestStatus("installing"); // reuse "installing" as "testing"
    setTestError(null);

    try {
      await invoke("test_api_connection", {
        baseUrl: apiBaseUrl || selectedProvider.baseUrl,
        apiKey,
        model: apiModel || selectedProvider.defaultModel,
      });
      setApiTestStatus("success");
    } catch (err) {
      setApiTestStatus("error");
      setTestError(String(err));
    }
  }, [selectedProvider, apiKey, apiBaseUrl, apiModel, setApiTestStatus]);

  // Save config and restart gateway so changes take effect
  const saveConfig = useCallback(async () => {
    if (!selectedProvider || !apiKey) return;
    await invoke("configure_api", {
      provider: selectedProvider.openclawProvider,
      apiKey,
      baseUrl: apiBaseUrl || selectedProvider.baseUrl,
      model: apiModel || selectedProvider.defaultModel,
    });
    // Restart gateway so new config takes effect (best-effort)
    try {
      await invoke("restart_openclaw_gateway");
    } catch {
      // Gateway may not be running yet; that's fine
    }
  }, [selectedProvider, apiKey, apiBaseUrl, apiModel]);

  const handleSkip = () => {
    goToStep(5);
  };

  const [saveError, setSaveError] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  const handleNext = async () => {
    if (selectedProvider && !apiKey) return;
    if (isSaving) return;
    setIsSaving(true);
    setSaveError(null);
    try {
      await saveConfig();
      goToStep(5);
    } catch (err) {
      setSaveError(String(err));
    } finally {
      setIsSaving(false);
    }
  };

  const handleBack = () => {
    goToStep(3);
  };

  const isTesting = apiTestStatus === "installing";

  return (
    <div className="apiconfig-page">
      <div className="apiconfig-header">
        <h1>{t("apiConfig.title")}</h1>
        <p>{t("apiConfig.description")}</p>
      </div>

      <div className="apiconfig-content">
        {/* Provider cards */}
        <div className="apiconfig-providers">
          {AI_PROVIDERS.map((provider) => {
            const Icon = PROVIDER_ICONS[provider.id] ?? Bot;
            return (
              <div
                key={provider.id}
                className={clsx(
                  "apiconfig-provider",
                  selectedProvider?.id === provider.id && "selected"
                )}
                onClick={() => {
                  setSelectedProvider(provider);
                  setApiTestStatus("idle");
                  // Scroll to form after React renders
                  requestAnimationFrame(() => {
                    formRef.current?.scrollIntoView({ behavior: "smooth", block: "start" });
                  });
                }}
              >
                <div className="apiconfig-provider-top">
                  <div className="apiconfig-provider-icon">
                    <Icon />
                  </div>
                  <span className="apiconfig-provider-name">
                    {t(provider.name)}
                  </span>
                  <div className="apiconfig-provider-badges">
                    {provider.recommended && (
                      <span className="apiconfig-badge recommended">
                        {t("apiConfig.recommended")}
                      </span>
                    )}
                    {provider.needsProxy && (
                      <span className="apiconfig-badge proxy">
                        {t("apiConfig.needsProxy")}
                      </span>
                    )}
                  </div>
                </div>
                <div className="apiconfig-provider-desc">
                  {t(provider.description)}
                </div>
                {PROVIDER_KEY_URLS[provider.id] && (
                  <a
                    className="apiconfig-provider-getkey"
                    onClick={(e) => {
                      e.stopPropagation();
                      openUrl(PROVIDER_KEY_URLS[provider.id]);
                    }}
                  >
                    {t("apiConfig.getKey")}
                    <ExternalLink size={12} />
                  </a>
                )}
              </div>
            );
          })}
        </div>

        {/* Configuration form (shown when a provider is selected) */}
        {selectedProvider && (
          <div className="apiconfig-form" ref={formRef}>
            <h3>
              {t("apiConfig.configureTitle", {
                provider: t(selectedProvider.name),
              })}
            </h3>

            <div className="apiconfig-field">
              <label className="apiconfig-label">{t("apiConfig.apiKeyInput")}</label>
              <input
                className="apiconfig-input"
                type="password"
                placeholder={t("apiConfig.apiKeyPlaceholder")}
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
              />
            </div>

            <div className="apiconfig-field">
              <label className="apiconfig-label">
                {t("apiConfig.baseUrlInput")}
              </label>
              <input
                className="apiconfig-input"
                type="text"
                placeholder={t("apiConfig.baseUrlPlaceholder")}
                value={apiBaseUrl}
                onChange={(e) => setApiBaseUrl(e.target.value)}
              />
            </div>

            <div className="apiconfig-field">
              <label className="apiconfig-label">
                {t("apiConfig.modelInput")}
              </label>
              <input
                className="apiconfig-input"
                type="text"
                placeholder={selectedProvider.defaultModel || t("apiConfig.modelPlaceholder")}
                value={apiModel}
                onChange={(e) => setApiModel(e.target.value)}
              />
              <span className="apiconfig-field-hint">{t("apiConfig.modelHint")}</span>
            </div>

            <div className="apiconfig-form-actions">
              <button
                className="apiconfig-test-btn"
                disabled={!apiKey || isTesting}
                onClick={handleTestConnection}
              >
                {isTesting
                  ? t("common.loading")
                  : t("apiConfig.testConnection")}
              </button>
            </div>

            {/* Test result */}
            {apiTestStatus === "success" && (
              <div className="apiconfig-test-result success">
                <CheckCircle2 size={16} />
                {t("apiConfig.testSuccess")}
              </div>
            )}
            {apiTestStatus === "error" && (
              <div className="apiconfig-test-result error">
                <XCircle size={16} />
                {testError || t("apiConfig.testFail")}
              </div>
            )}
            {isTesting && (
              <div className="apiconfig-test-result testing">
                <Loader2 size={16} className="animate-spin" />
                {t("common.loading")}
              </div>
            )}
          </div>
        )}

        {/* Save error */}
        {saveError && (
          <div className="apiconfig-test-result error">
            <XCircle size={16} />
            {saveError}
          </div>
        )}

        {/* Skip link (wizard only) */}
        {!isReconfig && (
          <div className="apiconfig-skip">
            <button className="apiconfig-skip-link" onClick={handleSkip}>
              {t("apiConfig.skip")}
            </button>
            <p className="apiconfig-skip-hint">{t("apiConfig.skipHint")}</p>
          </div>
        )}
      </div>

      {/* Navigation */}
      {isReconfig ? (
        <div className="apiconfig-actions">
          <button
            className="apiconfig-btn apiconfig-btn-primary btn-cta-glow"
            disabled={isSaving}
            onClick={async () => {
              if (isSaving) return;
              setIsSaving(true);
              setSaveError(null);
              try {
                await saveConfig();
                onDone?.();
              } catch (err) {
                setSaveError(String(err));
              } finally {
                setIsSaving(false);
              }
            }}
          >
            {isSaving ? (
              <><Loader2 size={16} className="animate-spin" /> {t("common.loading")}</>
            ) : (
              t("apiConfig.saveAndReturn")
            )}
          </button>
        </div>
      ) : (
        <div className="apiconfig-actions">
          <button className="apiconfig-btn apiconfig-btn-secondary" onClick={handleBack}>
            {t("btn.prev")}
          </button>
          <button
            className={clsx(
              "apiconfig-btn apiconfig-btn-primary",
              !(selectedProvider && !apiKey) && !isSaving && "btn-cta-glow"
            )}
            disabled={!!(selectedProvider && !apiKey) || isSaving}
            onClick={handleNext}
          >
            {isSaving ? (
              <><Loader2 size={16} className="animate-spin" /> {t("common.loading")}</>
            ) : (
              t("btn.next")
            )}
          </button>
        </div>
      )}
    </div>
  );
}
