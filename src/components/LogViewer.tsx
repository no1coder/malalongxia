import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import clsx from "clsx";
import type { LogEntry } from "../types";

interface LogViewerProps {
  readonly logs: readonly LogEntry[];
  readonly maxHeight?: string;
}

// Terminal-style log viewer with auto-scroll and color-coded log levels
export function LogViewer({ logs, maxHeight = "360px" }: LogViewerProps) {
  const { t } = useTranslation();
  const containerRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new logs arrive
  useEffect(() => {
    const container = containerRef.current;
    if (container) {
      container.scrollTop = container.scrollHeight;
    }
  }, [logs]);

  const formatTimestamp = (ts: number): string => {
    const date = new Date(ts);
    return date.toLocaleTimeString("en-US", { hour12: false });
  };

  if (logs.length === 0) {
    return (
      <div className="log-viewer" style={{ maxHeight }}>
        <div className="log-viewer__empty">{t("log.empty")}</div>
      </div>
    );
  }

  return (
    <div className="log-viewer" ref={containerRef} style={{ maxHeight }}>
      {logs.map((entry, index) => (
        <div
          key={`${entry.timestamp}-${index}`}
          className={clsx("log-viewer__line", `log-viewer__line--${entry.level}`)}
        >
          <span className="log-viewer__timestamp">
            {formatTimestamp(entry.timestamp)}
          </span>
          <span className="log-viewer__level">[{entry.level.toUpperCase()}]</span>
          <span className="log-viewer__message">{entry.message}</span>
        </div>
      ))}
    </div>
  );
}
