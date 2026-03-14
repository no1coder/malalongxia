import { describe, it, expect, beforeEach } from "vitest";
import { useInstallStore } from "./useInstallStore";
import type { EnvCheckItem } from "./useInstallStore";

// Reset store between tests
beforeEach(() => {
  useInstallStore.setState({
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
    nodeInstallMethod: "nvm",
    nodeInstallLogs: [],
    selectedMirror: null,
    mirrors: [],
    openclawInstallStatus: "idle",
    openclawVersion: null,
    openclawInstallLogs: [],
    selectedProvider: null,
    apiKey: "",
    apiBaseUrl: "",
    apiTestStatus: "idle",
  });
});

describe("useInstallStore", () => {
  describe("initial state", () => {
    it("has correct default values", () => {
      const state = useInstallStore.getState();
      expect(state.currentStep).toBe(0);
      expect(state.osType).toBe("unknown");
      expect(state.nodeVersion).toBeNull();
      expect(state.nodeRequired).toBe(false);
      expect(state.diskSpaceOk).toBe(true);
      expect(state.envChecks).toEqual([]);
      expect(state.nodeInstallStatus).toBe("idle");
      expect(state.nodeInstallMethod).toBe("nvm");
      expect(state.selectedMirror).toBeNull();
      expect(state.openclawInstallStatus).toBe("idle");
      expect(state.openclawVersion).toBeNull();
      expect(state.apiKey).toBe("");
      expect(state.apiTestStatus).toBe("idle");
    });
  });

  describe("setCurrentStep", () => {
    it("updates currentStep", () => {
      useInstallStore.getState().setCurrentStep(3);
      expect(useInstallStore.getState().currentStep).toBe(3);
    });
  });

  describe("setOsInfo", () => {
    it("sets both osType and osVersion", () => {
      useInstallStore.getState().setOsInfo("macos", "14.0");
      const state = useInstallStore.getState();
      expect(state.osType).toBe("macos");
      expect(state.osVersion).toBe("14.0");
    });
  });

  describe("setNodeVersion", () => {
    it("sets node version string", () => {
      useInstallStore.getState().setNodeVersion("v22.22.0");
      expect(useInstallStore.getState().nodeVersion).toBe("v22.22.0");
    });

    it("sets node version to null", () => {
      useInstallStore.getState().setNodeVersion("v22.22.0");
      useInstallStore.getState().setNodeVersion(null);
      expect(useInstallStore.getState().nodeVersion).toBeNull();
    });
  });

  describe("setNodeRequired", () => {
    it("marks node as required", () => {
      useInstallStore.getState().setNodeRequired(true);
      expect(useInstallStore.getState().nodeRequired).toBe(true);
    });
  });

  describe("setNpmVersion", () => {
    it("sets npm version", () => {
      useInstallStore.getState().setNpmVersion("10.0.0");
      expect(useInstallStore.getState().npmVersion).toBe("10.0.0");
    });
  });

  describe("setDiskSpaceOk", () => {
    it("sets disk space status", () => {
      useInstallStore.getState().setDiskSpaceOk(false);
      expect(useInstallStore.getState().diskSpaceOk).toBe(false);
    });
  });

  describe("env checks", () => {
    const mockChecks: readonly EnvCheckItem[] = [
      { id: "os", name: "OS", status: "pending", detail: "" },
      { id: "node", name: "Node", status: "pending", detail: "" },
    ];

    it("setEnvChecks replaces the entire list", () => {
      useInstallStore.getState().setEnvChecks(mockChecks);
      expect(useInstallStore.getState().envChecks).toEqual(mockChecks);
    });

    it("updateEnvCheck updates a specific check immutably", () => {
      useInstallStore.getState().setEnvChecks(mockChecks);
      const originalChecks = useInstallStore.getState().envChecks;

      useInstallStore.getState().updateEnvCheck("os", "passed", "macOS 14.0");
      const updatedChecks = useInstallStore.getState().envChecks;

      // Original array not mutated
      expect(originalChecks).toEqual(mockChecks);

      // Updated correctly
      expect(updatedChecks[0]).toEqual({
        id: "os",
        name: "OS",
        status: "passed",
        detail: "macOS 14.0",
      });
      // Other items unchanged
      expect(updatedChecks[1]).toEqual(mockChecks[1]);
    });

    it("updateEnvCheck does nothing for unknown id", () => {
      useInstallStore.getState().setEnvChecks(mockChecks);
      useInstallStore.getState().updateEnvCheck("unknown", "passed", "test");
      expect(useInstallStore.getState().envChecks).toEqual(mockChecks);
    });

    it("setEnvCheckComplete marks checks as complete", () => {
      useInstallStore.getState().setEnvCheckComplete(true);
      expect(useInstallStore.getState().envCheckComplete).toBe(true);
    });
  });

  describe("node install", () => {
    it("setNodeInstallStatus updates status", () => {
      useInstallStore.getState().setNodeInstallStatus("installing");
      expect(useInstallStore.getState().nodeInstallStatus).toBe("installing");
    });

    it("setNodeInstallMethod switches method", () => {
      useInstallStore.getState().setNodeInstallMethod("direct");
      expect(useInstallStore.getState().nodeInstallMethod).toBe("direct");
    });

    it("addNodeInstallLog appends log immutably", () => {
      const log1 = { timestamp: 1000, level: "info" as const, message: "Starting" };
      const log2 = { timestamp: 2000, level: "error" as const, message: "Failed" };

      useInstallStore.getState().addNodeInstallLog(log1);
      const logsAfterFirst = useInstallStore.getState().nodeInstallLogs;
      expect(logsAfterFirst).toHaveLength(1);
      expect(logsAfterFirst[0]).toEqual(log1);

      useInstallStore.getState().addNodeInstallLog(log2);
      const logsAfterSecond = useInstallStore.getState().nodeInstallLogs;
      expect(logsAfterSecond).toHaveLength(2);

      // First array not mutated
      expect(logsAfterFirst).toHaveLength(1);
    });
  });

  describe("mirror selection", () => {
    it("setSelectedMirror selects a mirror", () => {
      const mirror = { name: "aliyun", url: "https://npmmirror.com", type: "npm" as const };
      useInstallStore.getState().setSelectedMirror(mirror);
      expect(useInstallStore.getState().selectedMirror).toEqual(mirror);
    });

    it("setSelectedMirror clears selection with null", () => {
      const mirror = { name: "aliyun", url: "https://npmmirror.com", type: "npm" as const };
      useInstallStore.getState().setSelectedMirror(mirror);
      useInstallStore.getState().setSelectedMirror(null);
      expect(useInstallStore.getState().selectedMirror).toBeNull();
    });

    it("setMirrors sets mirror list", () => {
      const mirrors = [
        { name: "a", url: "https://a.com", type: "npm" as const },
        { name: "b", url: "https://b.com", type: "npm" as const },
      ];
      useInstallStore.getState().setMirrors(mirrors);
      expect(useInstallStore.getState().mirrors).toEqual(mirrors);
    });
  });

  describe("openclaw install", () => {
    it("setOpenclawInstallStatus updates status", () => {
      useInstallStore.getState().setOpenclawInstallStatus("success");
      expect(useInstallStore.getState().openclawInstallStatus).toBe("success");
    });

    it("setOpenclawVersion stores version", () => {
      useInstallStore.getState().setOpenclawVersion("1.2.3");
      expect(useInstallStore.getState().openclawVersion).toBe("1.2.3");
    });

    it("addOpenclawInstallLog appends log immutably", () => {
      const log = { timestamp: 1000, level: "info" as const, message: "Installing..." };
      useInstallStore.getState().addOpenclawInstallLog(log);
      expect(useInstallStore.getState().openclawInstallLogs).toEqual([log]);
    });
  });

  describe("API configuration", () => {
    it("setSelectedProvider sets provider and auto-fills baseUrl", () => {
      const provider = {
        id: "zhipu",
        name: "ZhiPu",
        description: "desc",
        baseUrl: "https://open.bigmodel.cn/api/paas/v4",
        needsProxy: false,
        openclawProvider: "zai",
        defaultModel: "glm-5",
      };
      useInstallStore.getState().setSelectedProvider(provider);
      const state = useInstallStore.getState();
      expect(state.selectedProvider).toEqual(provider);
      expect(state.apiBaseUrl).toBe("https://open.bigmodel.cn/api/paas/v4");
    });

    it("setSelectedProvider(null) clears baseUrl", () => {
      useInstallStore.getState().setApiBaseUrl("https://example.com");
      useInstallStore.getState().setSelectedProvider(null);
      expect(useInstallStore.getState().apiBaseUrl).toBe("");
    });

    it("setApiKey updates key", () => {
      useInstallStore.getState().setApiKey("sk-test-123");
      expect(useInstallStore.getState().apiKey).toBe("sk-test-123");
    });

    it("setApiBaseUrl overrides URL", () => {
      useInstallStore.getState().setApiBaseUrl("https://custom.api.com/v1");
      expect(useInstallStore.getState().apiBaseUrl).toBe("https://custom.api.com/v1");
    });

    it("setApiTestStatus updates test status", () => {
      useInstallStore.getState().setApiTestStatus("success");
      expect(useInstallStore.getState().apiTestStatus).toBe("success");
    });
  });
});
