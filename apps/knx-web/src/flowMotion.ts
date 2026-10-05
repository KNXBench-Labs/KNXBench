/** U21: whether the flow view may move: app Motion not Off and no OS reduce preference. */
// The rest of the app enforces `prefers-reduced-motion` in CSS (motion.ts).
// The flow view moves by script (solver steps, pulses), which CSS cannot
// stop, so it reads both sources itself and follows changes at once: a
// change mid-animation must stop the actual frames and timers.
import { useEffect, useState } from "react";

const REDUCE_QUERY = "(prefers-reduced-motion: reduce)";

export function motionAllowed(level: string | null, osReduce: boolean): boolean {
  return level !== "off" && !osReduce;
}

function read(): boolean {
  const level = document.documentElement.getAttribute("data-motion-level");
  const reduce = typeof matchMedia === "function" ? matchMedia(REDUCE_QUERY).matches : false;
  return motionAllowed(level, reduce);
}

export function useFlowMotion(): boolean {
  const [allowed, setAllowed] = useState(read);
  useEffect(() => {
    const update = () => setAllowed(read());
    const observer = new MutationObserver(update);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-motion-level"] });
    const query = typeof matchMedia === "function" ? matchMedia(REDUCE_QUERY) : null;
    query?.addEventListener("change", update);
    update();
    return () => {
      observer.disconnect();
      query?.removeEventListener("change", update);
    };
  }, []);
  return allowed;
}
