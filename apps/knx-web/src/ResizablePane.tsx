/** Pointer and keyboard resizing; pane content remains native document content. */
import { useEffect, useRef, useState, type ReactNode } from "react";
export default function ResizablePane(props: {
  label: string; side: "left" | "right"; initialWidth: number; min: number; max: number; children: ReactNode;
  onWidthCommit?: (width: number) => void;
}) {
  const clamp = (value: number) => Math.max(props.min, Math.min(props.max, value));
  const [width, setWidth] = useState(() => clamp(props.initialWidth));
  const widthRef = useRef(width);
  const drag = useRef<{ x: number; width: number } | null>(null);
  useEffect(() => {
    const next = clamp(props.initialWidth);
    widthRef.current = next;
    setWidth(next);
  }, [props.initialWidth, props.min, props.max]);
  const commit = () => {
    if (drag.current) props.onWidthCommit?.(widthRef.current);
    drag.current = null;
  };
  const direction = props.side === "left" ? 1 : -1;
  return <aside className={`workbench-pane workbench-pane-${props.side}`} style={{ width }}>
    <div className="workbench-pane-content">{props.children}</div>
    <div role="separator" aria-label={props.label} aria-orientation="vertical"
      aria-valuemin={props.min} aria-valuemax={props.max} aria-valuenow={width} tabIndex={0}
      className="pane-resizer"
      onKeyDown={(e) => {
        if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key)) return;
        e.preventDefault();
        const next = e.key === "Home" ? props.min : e.key === "End" ? props.max
          : clamp(widthRef.current + (e.key === "ArrowRight" ? 16 : -16) * direction);
        widthRef.current = next;
        setWidth(next);
        props.onWidthCommit?.(next);
      }}
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        drag.current = { x: e.clientX, width: widthRef.current };
        e.currentTarget.setPointerCapture(e.pointerId);
      }}
      onPointerMove={(e) => {
        if (!drag.current) return;
        const next = clamp(drag.current.width + (e.clientX - drag.current.x) * direction);
        widthRef.current = next;
        setWidth(next);
      }}
      onPointerUp={commit}
      onPointerCancel={commit}
      onLostPointerCapture={commit}
    />
  </aside>;
}
