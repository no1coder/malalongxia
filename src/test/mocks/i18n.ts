import { vi } from "vitest";

// Mock react-i18next: t() returns the key for predictable assertions
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown>) => {
      if (opts) {
        // Simple interpolation for test assertions
        let result = key;
        for (const [k, v] of Object.entries(opts)) {
          result += ` ${k}=${v}`;
        }
        return result;
      }
      return key;
    },
    i18n: {
      language: "zh-CN",
      changeLanguage: vi.fn(),
    },
  }),
  initReactI18next: {
    type: "3rdParty",
    init: vi.fn(),
  },
}));
