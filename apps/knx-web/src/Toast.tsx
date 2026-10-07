/** Renders the active toast queue as a dismissible alert/status stack. */
import type { ToastEntry } from "./toast";
import { useTranslate } from "./i18n";
import AchievementBadge from "./AchievementBadge";

export default function ToastStack(props: { toasts: ToastEntry[]; onDismiss: (id: number) => void }) {
  const t = useTranslate();
  if (props.toasts.length === 0) return null;
  return (
    <div className="toast-stack">
      {props.toasts.map((toast) => (
        <div
          key={toast.id}
          role={toast.kind === "error" ? "alert" : "status"}
          className={`toast toast--${toast.kind}`}
        >
          {toast.achievement && <AchievementBadge tier={toast.achievement.tier} glyph={toast.achievement.glyph} />}
          <div className="toast-body">
            {toast.achievement && <span className="toast-label">{t("achievements.unlockedLabel")}</span>}
            <span className={toast.achievement ? "toast-title" : undefined}>{toast.message}</span>
            {toast.achievement && <span className="toast-detail">{toast.achievement.description}</span>}
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
          <button aria-label={t("toast.dismiss")} onClick={() => props.onDismiss(toast.id)}>
            ×
          </button>
        </div>
      ))}
    </div>
  );
}
