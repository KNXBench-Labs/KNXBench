/** Mounts the real toast queue and view with offline controls for browser regressions. */
import { createRoot } from "react-dom/client";
import ToastStack from "../src/Toast";
import { useToasts } from "../src/toast";
import { setSetting } from "../src/settingsStore";
import { UI_LANGUAGE_STORAGE_KEY } from "../src/uiLanguage";
import { useThemeId } from "../src/theme";
import { useMotion } from "../src/motion";
import "../src/styles.css";

const params = new URLSearchParams(location.search);
setSetting(UI_LANGUAGE_STORAGE_KEY, params.get("lang") === "de" ? "de" : "en");
setSetting("theme", params.get("theme") ?? "graphite");
setSetting("motionLevel", params.get("motion") ?? "standard");

function Fixture() {
  useThemeId();
  useMotion();
  const queue = useToasts();
  return <main>
    <button onClick={() => queue.pushFun("Project saved. The relays may now relax.")}>Status</button>
    <button onClick={() => queue.pushError("Save refused: destination is read-only.")}>Error</button>
    <button onClick={() => queue.pushFun("x".repeat(240))}>Long status</button>
    <button onClick={() => queue.pushAchievements([
      { title: "Night Shift", description: "Saved at night.", tier: "gold", glyph: "moon" },
    ], (n) => `+${n} more`)}>Achievement</button>
    <ToastStack toasts={queue.toasts} onDismiss={queue.dismiss} onExited={queue.finishExit} />
  </main>;
}

createRoot(document.getElementById("root")!).render(<Fixture />);
