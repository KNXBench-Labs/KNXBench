/** Independently foldable navigation content that retains mounted editor and tree state. */
import { useId, useState, type ReactNode } from "react";

/**
 * `lazy` mounts the content only while the section is open: for read-only
 * lists that hold no draft and would otherwise render rows nobody sees.
 */
export default function NavigationSection({ title, children, grow = false, revealKey, lazy = false }: {
  title: string; children: ReactNode; grow?: boolean; revealKey?: number; lazy?: boolean;
}) {
  const id = useId();
  const [collapsed, setCollapsed] = useState(revealKey === undefined);
  const [lastReveal, setLastReveal] = useState(revealKey);
  // A deliberate search reveal reopens the explorer, not unrelated sections.
  if (lastReveal !== revealKey) { setLastReveal(revealKey); if (revealKey !== undefined) setCollapsed(false); }
  return <section className={`navigation-section${grow ? " navigation-section--grow" : ""}`}
    data-collapsed={collapsed}>
    <button type="button" className="navigation-section-toggle" aria-expanded={!collapsed}
      aria-controls={id} onClick={() => setCollapsed(value => !value)}>
      <span aria-hidden="true">{collapsed ? "▸" : "▾"}</span>{title}
    </button>
    <div id={id} className="navigation-section-content" hidden={collapsed}>{lazy && collapsed ? null : children}</div>
  </section>;
}
