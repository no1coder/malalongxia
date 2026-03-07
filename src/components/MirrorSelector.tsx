import { useTranslation } from "react-i18next";
import { Gauge, CheckCircle, AlertCircle, Radio, Loader } from "lucide-react";
import clsx from "clsx";
import type { Mirror } from "../types";

interface MirrorSelectorProps {
  readonly mirrors: readonly Mirror[];
  readonly selected: string;
  readonly onSelect: (url: string) => void;
  readonly onTest: () => void;
  readonly testing?: boolean;
}

// Mirror source selector with latency display and speed test
export function MirrorSelector({
  mirrors,
  selected,
  onSelect,
  onTest,
  testing = false,
}: MirrorSelectorProps) {
  const { t } = useTranslation();

  const getLatencyColor = (latency: number | undefined): string => {
    if (latency === undefined) return "var(--color-text-muted)";
    if (latency < 100) return "var(--color-success)";
    if (latency < 300) return "var(--color-warning)";
    return "var(--color-error)";
  };

  const getLatencyText = (latency: number | undefined): string => {
    if (latency === undefined) return t("mirror.untested");
    return `${latency}ms`;
  };

  const getStatusIcon = (latency: number | undefined) => {
    if (latency === undefined) return <Radio size={16} />;
    if (latency < 300) return <CheckCircle size={16} color="var(--color-success)" />;
    return <AlertCircle size={16} color="var(--color-warning)" />;
  };

  return (
    <div className="mirror-selector">
      <div className="mirror-selector__list">
        {mirrors.map((mirror) => (
          <button
            key={mirror.url}
            type="button"
            className={clsx("mirror-selector__item", {
              "mirror-selector__item--selected": selected === mirror.url,
            })}
            onClick={() => onSelect(mirror.url)}
          >
            <div className="mirror-selector__radio">
              <div
                className={clsx("mirror-selector__radio-dot", {
                  "mirror-selector__radio-dot--active": selected === mirror.url,
                })}
              />
            </div>

            <div className="mirror-selector__info">
              <span className="mirror-selector__name">{mirror.name}</span>
              <span className="mirror-selector__url">{mirror.url}</span>
            </div>

            <div className="mirror-selector__status">
              {getStatusIcon(mirror.latency)}
              <span
                className="mirror-selector__latency"
                style={{ color: getLatencyColor(mirror.latency) }}
              >
                {getLatencyText(mirror.latency)}
              </span>
            </div>
          </button>
        ))}
      </div>

      <button
        type="button"
        className="mirror-selector__test-btn"
        onClick={onTest}
        disabled={testing}
      >
        {testing ? (
          <Loader size={16} className="mirror-selector__spinner" />
        ) : (
          <Gauge size={16} />
        )}
        <span>{testing ? t("mirror.testing") : t("mirror.testSpeed")}</span>
      </button>
    </div>
  );
}
