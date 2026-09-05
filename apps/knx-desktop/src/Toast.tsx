import type { ToastEntry } from "./toast";

export default function ToastStack(props: { toasts: ToastEntry[]; onDismiss: (id: number) => void }) {
  if (props.toasts.length === 0) return null;
  return (
    <div className="toast-stack">
      {props.toasts.map((t) => (
        <div key={t.id} role={t.kind === "error" ? "alert" : "status"} className={`toast toast--${t.kind}`}>
          <span>{t.message}</span>
          <button aria-label="Dismiss" onClick={() => props.onDismiss(t.id)}>
            ×
          </button>
        </div>
      ))}
    </div>
  );
}
