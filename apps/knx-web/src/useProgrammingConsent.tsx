/** The one door every programming feature goes through: ask unless consent is remembered. */
// Usage, for the first feature that programs a device:
//
//   const consent = useProgrammingConsent();
//   …
//   if (!(await consent.request("1.1.67 — Switch actuator"))) return;
//   await api.programDevice(…);
//   …
//   return <>{…}{consent.dialog}</>;
//
// `request()` resolves `true` only after an explicit confirmation, or when
// a consent remembered for exactly this build's release stage exists. It
// resolves `false` on Cancel, Escape, the backdrop, or when the component
// unmounts with the question still open — never "yes" by default.
//
// The version is fetched fresh for every request rather than cached for
// the session: the stage decides whether a remembered answer still
// applies, and a stale guess about which build is running is exactly
// what this must not act on. A server that does not answer yields the
// `unknown` stage, which always asks.
import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import * as api from "./api";
import ProgrammingConsentDialog from "./ProgrammingConsentDialog";
import {
  isProgrammingConsentRemembered,
  releaseStageOf,
  rememberProgrammingConsent,
  type ReleaseStage,
} from "./programmingConsent";

interface Pending {
  stage: ReleaseStage;
  version: string | null;
  target: string;
  resolve: (confirmed: boolean) => void;
}

export interface ProgrammingConsent {
  /** Asks (or honours a remembered answer); resolves `true` only for a yes. */
  request: (target: string) => Promise<boolean>;
  /** The dialog while a question is open, otherwise `null`. Render it. */
  dialog: ReactNode;
}

async function runningVersion(): Promise<string | null> {
  try {
    return (await api.serverVersion()).version;
  } catch {
    return null;
  }
}

export function useProgrammingConsent(): ProgrammingConsent {
  const [pending, setPending] = useState<Pending | null>(null);
  const pendingRef = useRef<Pending | null>(null);
  const mounted = useRef(true);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      // A question nobody can answer any more is a "no".
      pendingRef.current?.resolve(false);
      pendingRef.current = null;
    };
  }, []);

  const settle = useCallback((confirmed: boolean) => {
    const current = pendingRef.current;
    if (!current) return;
    pendingRef.current = null;
    setPending(null);
    current.resolve(confirmed);
  }, []);

  const request = useCallback(async (target: string): Promise<boolean> => {
    // One question at a time: a second request while one is open is
    // refused rather than queued, so two writes cannot ride one "yes".
    if (pendingRef.current) return false;
    const version = await runningVersion();
    if (!mounted.current) return false;
    const stage = releaseStageOf(version);
    if (isProgrammingConsentRemembered(stage)) return true;
    if (pendingRef.current) return false;
    return new Promise<boolean>((resolve) => {
      const next: Pending = { stage, version, target, resolve };
      pendingRef.current = next;
      setPending(next);
    });
  }, []);

  const dialog = pending ? (
    <ProgrammingConsentDialog
      stage={pending.stage}
      version={pending.version}
      target={pending.target}
      onCancel={() => settle(false)}
      onConfirm={(remember) => {
        if (remember) rememberProgrammingConsent(pending.stage, pending.version);
        settle(true);
      }}
    />
  ) : null;

  return { request, dialog };
}
