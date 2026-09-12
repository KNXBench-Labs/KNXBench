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
          <span>{toast.message}</span>
          <button aria-label={t("toast.dismiss")} onClick={() => props.onDismiss(toast.id)}>
            ×
          </button>
        </div>
      ))}
    </div>
  );
}
