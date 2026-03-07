import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Mirror } from "../types";

// Remote mirror configuration returned by the Rust backend
interface RemoteMirrorConfig {
  readonly version: number;
  readonly updated_at: string;
  readonly node_mirrors: readonly MirrorEntry[];
  readonly npm_mirrors: readonly MirrorEntry[];
  readonly nvm_install_script: string | null;
  readonly node_version: string | null;
}

interface MirrorEntry {
  readonly name: string;
  readonly url: string;
  readonly enabled: boolean;
}

interface MirrorConfig {
  readonly nodeMirrors: readonly Mirror[];
  readonly npmMirrors: readonly Mirror[];
  readonly nvmInstallScript: string | null;
  readonly nodeVersion: string | null;
  readonly isLoading: boolean;
  readonly isRemote: boolean;
}

function toMirrors(entries: readonly MirrorEntry[], type: Mirror["type"]): Mirror[] {
  return entries.map((entry) => ({
    name: entry.name,
    url: entry.url,
    type,
  }));
}

/**
 * Fetch mirror configuration from remote (malalongxia.com/yuan.json)
 * with automatic local fallback if the remote is unreachable.
 */
export function useMirrorConfig(): MirrorConfig {
  const [config, setConfig] = useState<MirrorConfig>({
    nodeMirrors: [],
    npmMirrors: [],
    nvmInstallScript: null,
    nodeVersion: null,
    isLoading: true,
    isRemote: false,
  });

  useEffect(() => {
    let cancelled = false;

    invoke<RemoteMirrorConfig>("fetch_mirror_config")
      .then((remote) => {
        if (cancelled) return;
        setConfig({
          nodeMirrors: toMirrors(remote.node_mirrors, "node"),
          npmMirrors: toMirrors(remote.npm_mirrors, "npm"),
          nvmInstallScript: remote.nvm_install_script ?? null,
          nodeVersion: remote.node_version ?? null,
          isLoading: false,
          isRemote: remote.version > 0,
        });
      })
      .catch(() => {
        if (cancelled) return;
        // Should not reach here — Rust command always returns Ok with fallback
        setConfig((prev) => ({ ...prev, isLoading: false }));
      });

    return () => {
      cancelled = true;
    };
  }, []);

  return config;
}
