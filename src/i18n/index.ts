import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import zhCN from "./locales/zh-CN.json";
import enUS from "./locales/en-US.json";

// Supported language resources
const resources = {
  "zh-CN": { translation: zhCN },
  "en-US": { translation: enUS },
} as const;

// Detect stored language preference or fall back to zh-CN
const getStoredLanguage = (): string => {
  try {
    const stored = localStorage.getItem("app-language");
    if (stored === "zh-CN" || stored === "en-US") {
      return stored;
    }
  } catch {
    // localStorage may be unavailable in certain contexts
  }
  return "zh-CN";
};

i18n.use(initReactI18next).init({
  resources,
  lng: getStoredLanguage(),
  fallbackLng: "zh-CN",
  interpolation: {
    // React already escapes values, no need for double escaping
    escapeValue: false,
  },
  // Flatten nested keys with dot notation (e.g. "welcome.title")
  keySeparator: ".",
  // Disable namespace separator since we use a single "translation" namespace
  nsSeparator: false,
});

export default i18n;
