import { create } from "zustand";
import type {
  Mirror,
  InstallStatus,
  OSType,
  LogEntry,
} from "../types";

// Environment check item status
export type CheckStatus = "pending" | "checking" | "passed" | "warning" | "failed";

export interface EnvCheckItem {
  readonly id: string;
  readonly name: string;
  readonly status: CheckStatus;
  readonly detail: string;
}

// AI provider card info
export interface AIProviderOption {
  readonly id: string;
  readonly name: string;
  readonly description: string;
  readonly baseUrl: string;
  readonly needsProxy: boolean;
  readonly recommended?: boolean;
  // OpenClaw provider ID used in models.providers.<openclawProvider>
  readonly openclawProvider: string;
  // Default model ref in "provider/model" format
  readonly defaultModel: string;
}

interface InstallState {
  // Current wizard step (0-5)
  readonly currentStep: number;

  // Environment check results
  readonly osType: OSType;
  readonly osVersion: string;
  readonly nodeVersion: string | null;
  readonly nodeRequired: boolean;
  readonly npmVersion: string | null;
  readonly diskSpaceOk: boolean;
  readonly envChecks: readonly EnvCheckItem[];
  readonly envCheckComplete: boolean;

  // Node install
  readonly nodeInstallStatus: InstallStatus;
  readonly nodeInstallMethod: "nvm" | "direct";
  readonly nodeInstallLogs: readonly LogEntry[];

  // Mirror selection
  readonly selectedMirror: Mirror | null;
  readonly mirrors: readonly Mirror[];

  // OpenClaw install
  readonly openclawInstallStatus: InstallStatus;
  readonly openclawVersion: string | null;
  readonly openclawInstallLogs: readonly LogEntry[];

  // API configuration
  readonly selectedProvider: AIProviderOption | null;
  readonly apiKey: string;
  readonly apiBaseUrl: string;
  readonly apiModel: string;
  readonly apiTestStatus: InstallStatus;

  // Actions
  readonly setCurrentStep: (step: number) => void;
  readonly setOsInfo: (osType: OSType, osVersion: string) => void;
  readonly setNodeVersion: (version: string | null) => void;
  readonly setNodeRequired: (required: boolean) => void;
  readonly setNpmVersion: (version: string | null) => void;
  readonly setDiskSpaceOk: (ok: boolean) => void;
  readonly setEnvChecks: (checks: readonly EnvCheckItem[]) => void;
  readonly updateEnvCheck: (id: string, status: CheckStatus, detail: string) => void;
  readonly setEnvCheckComplete: (complete: boolean) => void;
  readonly setNodeInstallStatus: (status: InstallStatus) => void;
  readonly setNodeInstallMethod: (method: "nvm" | "direct") => void;
  readonly addNodeInstallLog: (entry: LogEntry) => void;
  readonly setSelectedMirror: (mirror: Mirror | null) => void;
  readonly setMirrors: (mirrors: readonly Mirror[]) => void;
  readonly setOpenclawInstallStatus: (status: InstallStatus) => void;
  readonly setOpenclawVersion: (version: string | null) => void;
  readonly addOpenclawInstallLog: (entry: LogEntry) => void;
  readonly setSelectedProvider: (provider: AIProviderOption | null) => void;
  readonly setApiKey: (key: string) => void;
  readonly setApiBaseUrl: (url: string) => void;
  readonly setApiModel: (model: string) => void;
  readonly setApiTestStatus: (status: InstallStatus) => void;
  readonly resetAll: () => void;
}

export const useInstallStore = create<InstallState>((set) => ({
  currentStep: 0,

  osType: "unknown",
  osVersion: "",
  nodeVersion: null,
  nodeRequired: false,
  npmVersion: null,
  diskSpaceOk: true,
  envChecks: [],
  envCheckComplete: false,

  nodeInstallStatus: "idle",
  nodeInstallMethod: "direct",
  nodeInstallLogs: [],

  selectedMirror: null,
  mirrors: [],

  openclawInstallStatus: "idle",
  openclawVersion: null,
  openclawInstallLogs: [],

  selectedProvider: null,
  apiKey: "",
  apiBaseUrl: "",
  apiModel: "",
  apiTestStatus: "idle",

  setCurrentStep: (step) => set({ currentStep: step }),
  setOsInfo: (osType, osVersion) => set({ osType, osVersion }),
  setNodeVersion: (version) => set({ nodeVersion: version }),
  setNodeRequired: (required) => set({ nodeRequired: required }),
  setNpmVersion: (version) => set({ npmVersion: version }),
  setDiskSpaceOk: (ok) => set({ diskSpaceOk: ok }),
  setEnvChecks: (checks) => set({ envChecks: checks }),
  updateEnvCheck: (id, status, detail) =>
    set((state) => ({
      envChecks: state.envChecks.map((check) =>
        check.id === id ? { ...check, status, detail } : check
      ),
    })),
  setEnvCheckComplete: (complete) => set({ envCheckComplete: complete }),
  setNodeInstallStatus: (status) => set({ nodeInstallStatus: status }),
  setNodeInstallMethod: (method) => set({ nodeInstallMethod: method }),
  addNodeInstallLog: (entry) =>
    set((state) => ({
      nodeInstallLogs: [...state.nodeInstallLogs, entry],
    })),
  setSelectedMirror: (mirror) => set({ selectedMirror: mirror }),
  setMirrors: (mirrors) => set({ mirrors: mirrors }),
  setOpenclawInstallStatus: (status) => set({ openclawInstallStatus: status }),
  setOpenclawVersion: (version) => set({ openclawVersion: version }),
  addOpenclawInstallLog: (entry) =>
    set((state) => ({
      openclawInstallLogs: [...state.openclawInstallLogs, entry],
    })),
  setSelectedProvider: (provider) =>
    set({
      selectedProvider: provider,
      apiKey: "",
      apiBaseUrl: provider?.baseUrl ?? "",
      apiModel: provider?.defaultModel ?? "",
      apiTestStatus: "idle",
    }),
  setApiKey: (key) => set({ apiKey: key }),
  setApiBaseUrl: (url) => set({ apiBaseUrl: url }),
  setApiModel: (model) => set({ apiModel: model }),
  setApiTestStatus: (status) => set({ apiTestStatus: status }),
  resetAll: () =>
    set({
      currentStep: 0,
      osType: "unknown",
      osVersion: "",
      nodeVersion: null,
      nodeRequired: false,
      npmVersion: null,
      diskSpaceOk: true,
      envChecks: [],
      envCheckComplete: false,
      nodeInstallStatus: "idle",
      nodeInstallMethod: "direct",
      nodeInstallLogs: [],
      selectedMirror: null,
      mirrors: [],
      openclawInstallStatus: "idle",
      openclawVersion: null,
      openclawInstallLogs: [],
      selectedProvider: null,
      apiKey: "",
      apiBaseUrl: "",
      apiModel: "",
      apiTestStatus: "idle",
    }),
}));
