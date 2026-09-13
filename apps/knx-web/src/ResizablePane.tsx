/** Pointer and keyboard resizing; pane content remains native document content. */
import { useRef, useState, type ReactNode } from "react";
export default function ResizablePane(props: {
  label: string; side: "left" | "right"; initialWidth: number; min: number; max: number; children: ReactNode;
}) {
  const [width, setWidth] = useState(props.initialWidth);
  const drag = useRef<{ x: number; width: number } | null>(null);
  const clamp = (value: number) => Math.max(props.min, Math.min(props.max, value));
  const direction = props.side === "left" ? 1 : -1;
  return <aside className={`workbench-pane workbench-pane-${props.side}`} style={{ width }}>
    <div className="workbench-pane-content">{props.children}</div>
    <div role="separator" aria-label={props.label} aria-orientation="vertical"
      aria-valuemin={props.min} aria-valuemax={props.max} aria-valuenow={width} tabIndex={0}
      className="pane-resizer"
      onKeyDown={(e) => {
        if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key)) return;
        e.preventDefault();
        setWidth((w) => e.key === "Home" ? props.min : e.key === "End" ? props.max : clamp(w + (e.key === "ArrowRight" ? 16 : -16) * direction));
      }}
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        drag.current = { x: e.clientX, width };
        e.currentTarget.setPointerCapture(e.pointerId);
      }}
      onPointerMove={(e) => { if (drag.current) setWidth(clamp(drag.current.width + (e.clientX - drag.current.x) * direction)); }}
      onPointerUp={() => { drag.current = null; }}
      onPointerCancel={() => { drag.current = null; }}
      onLostPointerCapture={() => { drag.current = null; }}
    />
  </aside>;
}
