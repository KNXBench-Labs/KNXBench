/** Renders the active toast queue as a dismissible alert/status stack. */
import type { ToastEntry } from "./toast";
import { useTranslate } from "./i18n";

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
          <div className="toast-body">
            <span>{toast.message}</span>
            {/* §66/§67 disclosure (fix round 2, B4): the joke wrapper above
                is translated, but the `{msg}` it quotes is the server's raw
                English text — stated here, where the reader meets it, not
                just in KNOWN_LIMITATIONS.md. Fun toasts carry no such
                quote, so only error toasts get this line. */}
            {toast.kind === "error" && (
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
