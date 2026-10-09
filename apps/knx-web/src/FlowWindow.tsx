/** Dedicated read-only flow window: subscribes to its source, never polls or opens a tunnel. */
import { useEffect, useState } from "react";
import TelegramFlowView from "./TelegramFlowView";
import { useFlowMirror } from "./flowChannel";
import { canReturnToMainWindow, focusMainWindow, returnToMainWindow } from "./diagnosticsWindow";
import { useTranslate } from "./i18n";
import { useThemeId } from "./theme";
import { useMotion } from "./motion";
import { useAppearance } from "./appearance";

export default function FlowWindow() {
  const t = useTranslate();
  useThemeId(); useMotion(); useAppearance();
  const owner = new URLSearchParams(window.location.search).get("source");
  const { state, feed, navigation } = useFlowMirror(owner);
  const [canReturn, setCanReturn] = useState(false);
  useEffect(() => {
    let cancelled = false;
    void canReturnToMainWindow().then(value => { if (!cancelled) setCanReturn(value); });
    return () => { cancelled = true; };
  }, [state.live]);
  useEffect(() => { document.title = t("flow.windowTitle"); }, [t]);
  return <main className="flow-window">
    <header className="flow-window-heading"><h1>{t("flow.windowTitle")}</h1>
      <button type="button" disabled={!canReturn} title={canReturn ? undefined : t("companion.noMainWindow")} onClick={() => void returnToMainWindow()}>{t("companion.backToMain")}</button></header>
    {!state.live && <p role="status">{t(state.revision ? "flow.ownerLost" : "flow.waitingOwner")}</p>}
    {state.paused && state.live && <p role="status">{t("flow.pausedOwner")}</p>}
    <TelegramFlowView key={feed.model ? `${feed.model.identity.serverIncarnation}:${feed.model.identity.sessionId}` : "empty"} feed={feed}
      navigation={{ ...navigation, open: async target => { const ok = await navigation.open(target); if (ok) await focusMainWindow(); return ok; } }} />
  </main>;
}
