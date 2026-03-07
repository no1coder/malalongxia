import { useTranslation } from "react-i18next";
import { Download, X } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";

interface AppUpdateInfo {
  readonly has_update: boolean;
  readonly current_version: string;
  readonly latest_version: string;
  readonly download_url: string;
  readonly release_notes: string;
}

interface UpdateBannerProps {
  readonly updateInfo: AppUpdateInfo;
  readonly onDismiss: () => void;
}

export function UpdateBanner({ updateInfo, onDismiss }: UpdateBannerProps) {
  const { t } = useTranslation();

  const handleDownload = async () => {
    try {
      await invoke("open_url", { url: updateInfo.download_url });
    } catch {
      window.open(updateInfo.download_url, "_blank");
    }
  };

  return (
    <div className="update-banner">
      <div className="update-banner__content">
        <Download size={16} className="update-banner__icon" />
        <span className="update-banner__text">
          {t("update.newVersion", { version: updateInfo.latest_version })}
          {updateInfo.release_notes && (
            <span className="update-banner__notes">
              {" — "}
              {updateInfo.release_notes}
            </span>
          )}
        </span>
      </div>
      <div className="update-banner__actions">
        <button
          className="update-banner__download-btn"
          onClick={handleDownload}
        >
          {t("update.download")}
        </button>
        <button
          className="update-banner__dismiss-btn"
          onClick={onDismiss}
          aria-label={t("common.close")}
        >
          <X size={14} />
        </button>
      </div>
    </div>
  );
}
