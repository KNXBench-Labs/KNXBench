/** Renders the active toast queue as a dismissible alert/status stack. */
import type { ToastEntry, ToastPauseReason } from "./toast";
import { useTranslate } from "./i18n";
import AchievementBadge from "./AchievementBadge";

/** The exit animation's name in `styles.css`; only its end removes a toast. */
const EXIT_ANIMATION = "knx-achievement-out";

export default function ToastStack(props: {
  toasts: ToastEntry[];
  onDismiss: (id: number) => void;
  onExited?: (id: number) => void;
  onOpen?: (id: number) => void;
  onPause?: (id: number, reason: ToastPauseReason) => void;
  onResume?: (id: number, reason: ToastPauseReason) => void;
  /** Cards waiting for a free slot; their lifetime has not started. */
  pendingCount?: number;
  /** Waiting cards retired on queue overflow; the session log keeps them. */
  queueDropped?: number;
}) {
  const t = useTranslate();
  if (props.toasts.length === 0) return null;
  return (
    <div className="toast-stack">
      {props.toasts.map((toast) => (
        <div
          key={toast.id}
          role={toast.kind === "error" ? "alert" : "status"}
          className={`toast toast--${toast.kind}${toast.leaving ? " toast--leaving" : ""}`}
          onMouseEnter={() => props.onPause?.(toast.id, "hover")}
          onMouseLeave={() => props.onResume?.(toast.id, "hover")}
          onFocus={() => props.onPause?.(toast.id, "focus")}
          onBlur={(event) => {
            if (!event.currentTarget.contains(event.relatedTarget as Node | null)) props.onResume?.(toast.id, "focus");
          }}
          onAnimationEnd={(event) => {
            // A child's animation (the achievement badge) bubbles here too.
            if (event.target === event.currentTarget && toast.leaving && event.animationName === EXIT_ANIMATION) props.onExited?.(toast.id);
          }}
        >
          {toast.achievement && <AchievementBadge tier={toast.achievement.tier} glyph={toast.achievement.glyph} />}
          {props.onOpen && <button type="button" className="toast-open" aria-label={t("toast.openMessage")} onClick={() => props.onOpen?.(toast.id)} />}
          <div className="toast-body">
            {toast.achievement && <span className="toast-label">{t("achievements.unlockedLabel")}</span>}
            <span className="toast-title">{toast.message}</span>
            {toast.achievement && <span className="toast-detail">{toast.achievement.description}</span>}
            {toast.releaseUrl && <a className="toast-release-link" href={toast.releaseUrl} target="_blank" rel="noopener noreferrer">{t("updates.releaseDetails")}</a>}
            {/* §66/§67 disclosure (fix round 2, B4; corrected round 3, B1):
                the joke wrapper above is translated, but when `serverText`
                is true the `{msg}` it quotes is the server's raw English
                text — stated here, where the reader meets it, not just in
                KNOWN_LIMITATIONS.md. Some error toasts (e.g. the companion-
                window ones) are fully-translated catalogue strings with no
                such quote, so `kind === "error"` alone is not enough to
                gate this line — `serverText` is what App.tsx sets per
                call, not what we infer from `kind`. */}
            {toast.kind === "error" && toast.serverText && (
              <span className="toast-hint">{t("toast.error.messageIsEnglish")}</span>
            )}
          </div>
          <button type="button" className="toast-dismiss" aria-label={t("toast.dismiss")} onClick={() => props.onDismiss(toast.id)}>
            ×
          </button>
        </div>
      ))}
      {!!props.pendingCount && <p className="toast-queue-note" role="status">{t("toast.pending", { count: props.pendingCount })}</p>}
      {!!props.queueDropped && <p className="toast-queue-note">{t("toast.queueDropped", { count: props.queueDropped })}</p>}
    </div>
  );
}
