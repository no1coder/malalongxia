import { describe, it, expect, vi, beforeEach } from "vitest";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

const { mockInvoke, mockOpenUrl } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
  mockOpenUrl: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown>) => {
      if (opts) return `${key} ${JSON.stringify(opts)}`;
      return key;
    },
    i18n: { language: "zh-CN", changeLanguage: vi.fn() },
  }),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mockInvoke }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: mockOpenUrl }));

import DashboardPage from "./DashboardPage";
import { renderWithRouter } from "../test/render";

const DEFAULT_PROPS = {
  currentVersion: "1.2.3",
  latestVersion: "1.2.3",
  needsUpdate: false,
  running: true,
  gatewayUrl: "http://127.0.0.1:18789/#token=abc",
  onReinstall: vi.fn(),
  onReconfigureApi: vi.fn(),

  onUninstall: vi.fn(),
};

describe("DashboardPage", () => {
  beforeEach(() => {
    mockInvoke.mockClear();
    mockOpenUrl.mockClear();
    DEFAULT_PROPS.onReinstall.mockClear();
    DEFAULT_PROPS.onReconfigureApi.mockClear();
    DEFAULT_PROPS.onUninstall.mockClear();
  });

  // --- Rendering ---

  it("renders dashboard title", () => {
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);
    expect(screen.getByText("dashboard.title")).toBeInTheDocument();
  });

  it("renders version info", () => {
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);
    // v1.2.3 appears twice: current version + latest version (both "1.2.3")
    const versionElements = screen.getAllByText("v1.2.3");
    expect(versionElements.length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("dashboard.version")).toBeInTheDocument();
  });

  it("shows up-to-date badge when no update needed", () => {
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);
    expect(screen.getByText("dashboard.upToDate")).toBeInTheDocument();
  });

  it("shows update available badge when update needed", () => {
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} needsUpdate={true} latestVersion="2.0.0" />);
    expect(screen.getByText("dashboard.updateAvailable")).toBeInTheDocument();
  });

  it("shows latest version when available", () => {
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} latestVersion="2.0.0" needsUpdate={true} />);
    expect(screen.getByText("v2.0.0")).toBeInTheDocument();
  });

  // --- Gateway Status ---

  it("shows running status when gateway is running", () => {
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);
    expect(screen.getByText("dashboard.gatewayRunning")).toBeInTheDocument();
  });

  it("shows checking status when gateway is not running initially", () => {
    // When running=false, the component starts checking status via invoke
    mockInvoke.mockResolvedValue({ installed: true, running: false, current_version: "1.0.0", latest_version: "1.0.0", needs_update: false, gateway_url: "" });
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} running={false} />);
    expect(screen.getByText("dashboard.checkingStatus")).toBeInTheDocument();
  });

  it("shows stopped status after check completes", async () => {
    mockInvoke.mockResolvedValue({ installed: true, running: false, current_version: "1.0.0", latest_version: "1.0.0", needs_update: false, gateway_url: "" });
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} running={false} />);

    await waitFor(() => {
      expect(screen.getByText("dashboard.gatewayStopped")).toBeInTheDocument();
    });
  });

  it("shows Open WebUI buttons when running", () => {
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);
    // "dashboard.openWebUI" appears twice: in the banner and in management section
    const buttons = screen.getAllByText("dashboard.openWebUI");
    expect(buttons.length).toBeGreaterThanOrEqual(1);
  });

  it("shows launch button when not running and check done", async () => {
    mockInvoke.mockResolvedValue({ installed: true, running: false, current_version: "1.0.0", latest_version: "1.0.0", needs_update: false, gateway_url: "" });
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} running={false} />);

    await waitFor(() => {
      expect(screen.getByText("dashboard.launch")).toBeInTheDocument();
    });
  });

  // --- Launch ---

  it("calls launch_openclaw on launch click", async () => {
    mockInvoke.mockResolvedValueOnce({ installed: true, running: false, current_version: "1.0.0", latest_version: null, needs_update: false, gateway_url: "" });
    mockInvoke.mockResolvedValueOnce("http://localhost:18789");
    const user = userEvent.setup();

    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} running={false} />);

    await waitFor(() => {
      expect(screen.getByText("dashboard.launch")).toBeInTheDocument();
    });

    await user.click(screen.getByText("dashboard.launch"));
    expect(mockInvoke).toHaveBeenCalledWith("launch_openclaw");
  });

  it("shows launch success message", async () => {
    mockInvoke.mockResolvedValueOnce({ installed: true, running: false, current_version: "1.0.0", latest_version: null, needs_update: false, gateway_url: "" });
    mockInvoke.mockResolvedValueOnce("http://localhost:18789");
    const user = userEvent.setup();

    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} running={false} />);

    await waitFor(() => {
      expect(screen.getByText("dashboard.launch")).toBeInTheDocument();
    });

    await user.click(screen.getByText("dashboard.launch"));
    await waitFor(() => {
      expect(screen.getByText("dashboard.launchSuccess")).toBeInTheDocument();
    });
  });

  it("shows launch error message on failure", async () => {
    mockInvoke.mockResolvedValueOnce({ installed: true, running: false, current_version: "1.0.0", latest_version: null, needs_update: false, gateway_url: "" });
    mockInvoke.mockRejectedValueOnce(new Error("Gateway timeout"));
    const user = userEvent.setup();

    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} running={false} />);

    await waitFor(() => {
      expect(screen.getByText("dashboard.launch")).toBeInTheDocument();
    });

    await user.click(screen.getByText("dashboard.launch"));
    await waitFor(() => {
      expect(screen.getByText("Error: Gateway timeout")).toBeInTheDocument();
    });
  });

  // --- Stop ---

  it("calls stop_openclaw_gateway on stop click", async () => {
    mockInvoke.mockResolvedValue("stopped");
    const user = userEvent.setup();

    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);

    await user.click(screen.getByText("dashboard.stopGateway"));
    expect(mockInvoke).toHaveBeenCalledWith("stop_openclaw_gateway");
  });

  it("shows stop success message", async () => {
    mockInvoke.mockResolvedValue("stopped");
    const user = userEvent.setup();

    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);

    await user.click(screen.getByText("dashboard.stopGateway"));
    await waitFor(() => {
      expect(screen.getByText("dashboard.stopSuccess")).toBeInTheDocument();
    });
  });

  // --- Restart ---

  it("calls restart_openclaw_gateway on restart click", async () => {
    mockInvoke.mockResolvedValue("http://127.0.0.1:18789");
    const user = userEvent.setup();

    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);

    await user.click(screen.getByText("dashboard.restart"));
    expect(mockInvoke).toHaveBeenCalledWith("restart_openclaw_gateway");
  });

  it("shows restart success message", async () => {
    mockInvoke.mockResolvedValue("restarted");
    const user = userEvent.setup();

    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);

    await user.click(screen.getByText("dashboard.restart"));
    await waitFor(() => {
      expect(screen.getByText("dashboard.restartSuccess")).toBeInTheDocument();
    });
  });

  // --- Update ---

  it("shows update button when update available", () => {
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} needsUpdate={true} latestVersion="2.0.0" />);
    expect(screen.getByText(/dashboard.updateTo/)).toBeInTheDocument();
  });

  it("does not show update button when up to date", () => {
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);
    expect(screen.queryByText(/dashboard.updateTo/)).not.toBeInTheDocument();
  });

  it("calls update_openclaw on update click", async () => {
    mockInvoke.mockResolvedValue("2.0.0");
    const user = userEvent.setup();

    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} needsUpdate={true} latestVersion="2.0.0" />);

    await user.click(screen.getByText(/dashboard.updateTo/));
    expect(mockInvoke).toHaveBeenCalledWith("update_openclaw");
  });

  it("shows update success and hides update button", async () => {
    mockInvoke.mockResolvedValue("2.0.0");
    const user = userEvent.setup();

    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} needsUpdate={true} latestVersion="2.0.0" />);

    await user.click(screen.getByText(/dashboard.updateTo/));
    await waitFor(() => {
      expect(screen.getByText(/dashboard.updateSuccess/)).toBeInTheDocument();
    });
    // Update button should be hidden after success
    expect(screen.queryByText(/dashboard.updateTo/)).not.toBeInTheDocument();
  });

  // --- Diagnostics ---

  it("calls openclaw_doctor on doctor click", async () => {
    mockInvoke.mockResolvedValue("All checks passed");
    const user = userEvent.setup();

    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);

    await user.click(screen.getByText("dashboard.doctor"));
    expect(mockInvoke).toHaveBeenCalledWith("openclaw_doctor");
  });

  it("shows doctor output after completion", async () => {
    mockInvoke.mockResolvedValue("[OK] Gateway is running\n[OK] Config valid");
    const user = userEvent.setup();

    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);

    await user.click(screen.getByText("dashboard.doctor"));
    await waitFor(() => {
      expect(screen.getByText(/Gateway is running/)).toBeInTheDocument();
    });
  });

  it("calls repair_openclaw on repair click", async () => {
    mockInvoke.mockResolvedValue("[OK] Fixed");
    const user = userEvent.setup();

    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);

    await user.click(screen.getByText("dashboard.repair"));
    expect(mockInvoke).toHaveBeenCalledWith("repair_openclaw");
  });

  it("shows repair output after completion", async () => {
    mockInvoke.mockResolvedValue("[OK] Set gateway.mode=local\n[OK] Installed gateway service");
    const user = userEvent.setup();

    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);

    await user.click(screen.getByText("dashboard.repair"));
    await waitFor(() => {
      expect(screen.getByText(/gateway.mode=local/)).toBeInTheDocument();
    });
  });

  // --- Other Actions ---

  it("calls onReconfigureApi when reconfigure clicked", async () => {
    const user = userEvent.setup();
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);

    await user.click(screen.getByText("dashboard.reconfigureApi"));
    expect(DEFAULT_PROPS.onReconfigureApi).toHaveBeenCalledTimes(1);
  });

  it("calls onReinstall when reinstall clicked and confirmed via dialog", async () => {
    const user = userEvent.setup();
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);

    // Click reinstall button to open confirm dialog
    await user.click(screen.getByText("dashboard.reinstall"));
    // Confirm dialog should appear with confirm text
    expect(screen.getByText("dashboard.reinstallConfirm")).toBeInTheDocument();
    // Click the confirm (reinstall) button in the dialog
    const reinstallButtons = screen.getAllByText("dashboard.reinstall");
    await user.click(reinstallButtons[reinstallButtons.length - 1]);
    expect(DEFAULT_PROPS.onReinstall).toHaveBeenCalledTimes(1);
  });

  it("renders view tutorial button", () => {
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);
    expect(screen.getByText("dashboard.viewTutorial")).toBeInTheDocument();
  });

  it("renders uninstall button", () => {
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);
    expect(screen.getByText("dashboard.uninstall")).toBeInTheDocument();
  });

  it("shows uninstall confirm dialog when uninstall button clicked", async () => {
    const user = userEvent.setup();
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);

    await user.click(screen.getByText("dashboard.uninstall"));
    expect(screen.getByText("dashboard.uninstallConfirm")).toBeInTheDocument();
  });

  it("dismisses uninstall dialog when cancel clicked", async () => {
    const user = userEvent.setup();
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);

    await user.click(screen.getByText("dashboard.uninstall"));
    expect(screen.getByText("dashboard.uninstallConfirm")).toBeInTheDocument();

    // Get all cancel buttons and click the one in the dialog
    const cancelButtons = screen.getAllByText("common.cancel");
    await user.click(cancelButtons[cancelButtons.length - 1]);
    expect(screen.queryByText("dashboard.uninstallConfirm")).not.toBeInTheDocument();
    expect(DEFAULT_PROPS.onUninstall).not.toHaveBeenCalled();
  });

  it("calls reset_installation and shows success when confirmed", async () => {
    mockInvoke.mockResolvedValue(undefined);
    const user = userEvent.setup();
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);

    await user.click(screen.getByText("dashboard.uninstall"));
    // Confirm in dialog — last button with label "dashboard.uninstall"
    const uninstallButtons = screen.getAllByText("dashboard.uninstall");
    await user.click(uninstallButtons[uninstallButtons.length - 1]);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("reset_installation");
    });

    // Success message should appear
    await waitFor(() => {
      expect(screen.getByText("dashboard.uninstallSuccess")).toBeInTheDocument();
    });
  });

  it("shows error message when uninstall fails", async () => {
    mockInvoke.mockRejectedValue("invoke error: Permission denied");
    const user = userEvent.setup();
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);

    await user.click(screen.getByText("dashboard.uninstall"));
    const uninstallButtons = screen.getAllByText("dashboard.uninstall");
    await user.click(uninstallButtons[uninstallButtons.length - 1]);

    await waitFor(() => {
      expect(screen.getByText("Permission denied")).toBeInTheDocument();
    });
    expect(DEFAULT_PROPS.onUninstall).not.toHaveBeenCalled();
  });

  // --- Version display logic ---

  it("shows question mark when version is null", () => {
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} currentVersion={null} />);
    expect(screen.getByText("v?")).toBeInTheDocument();
  });

  it("shows updated version after update", async () => {
    mockInvoke.mockResolvedValue("3.0.0");
    const user = userEvent.setup();

    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} needsUpdate={true} latestVersion="3.0.0" />);

    await user.click(screen.getByText(/dashboard.updateTo/));
    await waitFor(() => {
      // v3.0.0 appears in both current version and latest version rows
      const versionElements = screen.getAllByText("v3.0.0");
      expect(versionElements.length).toBeGreaterThanOrEqual(1);
    });
  });

  // --- Section titles ---

  it("renders management section title", () => {
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);
    expect(screen.getByText("dashboard.managementTitle")).toBeInTheDocument();
  });

  it("renders diagnostics section title", () => {
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);
    expect(screen.getByText("dashboard.diagnosticsTitle")).toBeInTheDocument();
  });

  it("renders actions section title", () => {
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} />);
    expect(screen.getByText("dashboard.actions")).toBeInTheDocument();
  });

  // --- Gateway URL display ---

  it("strips query params from displayed URL", () => {
    renderWithRouter(<DashboardPage {...DEFAULT_PROPS} gatewayUrl="http://127.0.0.1:18789/?token=secret" />);
    expect(screen.getByText("http://127.0.0.1:18789/")).toBeInTheDocument();
  });
});
