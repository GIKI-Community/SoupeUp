import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";

import { UpdateApi } from "@/api";
import { Button } from "@/components/ui/button";
import { useSettingsStore } from "@/stores";
import type { UpdateCheckResult } from "@/types";

const DISMISS_KEY = "cluster-runtime-update-dismissed";

function wasDismissedFor(version: string | null | undefined): boolean {
  if (!version) return false;
  try {
    return sessionStorage.getItem(DISMISS_KEY) === version;
  } catch {
    return false;
  }
}

function dismissFor(version: string) {
  try {
    sessionStorage.setItem(DISMISS_KEY, version);
  } catch {
    /* ignore */
  }
}

/** Startup check + dismissible banner when a newer GitHub release exists. */
export function UpdateBanner() {
  const autoCheck =
    useSettingsStore((s) => s.settings.autoCheckUpdates) ?? true;
  const [result, setResult] = useState<UpdateCheckResult | null>(null);
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    if (!autoCheck) return;
    let cancelled = false;
    void UpdateApi.check()
      .then((r) => {
        if (cancelled) return;
        if (r.updateAvailable && !wasDismissedFor(r.latestVersion)) {
          setResult(r);
        }
      })
      .catch(() => {
        /* silent on startup — Settings page surfaces errors */
      });
    return () => {
      cancelled = true;
    };
  }, [autoCheck]);

  if (!result?.updateAvailable || dismissed) return null;

  const latest = result.latestVersion ?? "new";

  return (
    <div className="flex items-center justify-between gap-4 border-b border-border/60 bg-muted/40 px-8 py-2.5 text-sm">
      <p className="text-foreground">
        <span className="font-medium">Update available:</span>{" "}
        <span className="text-muted-foreground">
          v{latest} (you have v{result.currentVersion})
        </span>
      </p>
      <div className="flex shrink-0 items-center gap-2">
        {result.releaseUrl && (
          <Button
            size="sm"
            onClick={() => {
              void openUrl(result.releaseUrl!);
            }}
          >
            View release
          </Button>
        )}
        <Button
          size="sm"
          variant="ghost"
          onClick={() => {
            dismissFor(latest);
            setDismissed(true);
          }}
        >
          Dismiss
        </Button>
      </div>
    </div>
  );
}
