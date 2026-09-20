/** Mounts the single React root as either the editing workspace or the diagnostics companion. */
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import AuthGate from "./AuthGate";
import DiagnosticsCompanion from "./DiagnosticsCompanion";
import { isCompanionView } from "./diagnosticsWindow";
import "@fontsource/space-grotesk/400.css";
import "@fontsource/space-grotesk/500.css";
import "@fontsource/space-grotesk/600.css";
import "@fontsource/space-grotesk/700.css";
import "@fontsource/inter/400.css";
import "@fontsource/inter/500.css";
import "@fontsource/inter/600.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";
import "./styles.css";

// One bundle, one origin, two roles. `?view=diagnostics` selects the
// read-only companion window (`DiagnosticsCompanion.tsx`); everything else
// is the one editing workspace. Branching here rather than inside `App`
// keeps the editor's whole module graph — and therefore every project
// mutation and the undo shortcuts — out of the companion entirely.
const companion = isCompanionView(window.location.search);

// `AuthGate` (ADR-0026) wraps both roles: the companion window is a second
// window on the same origin and so shares the session cookie, and a
// companion opened after a session expired deserves the same login screen
// rather than a panel quietly failing to poll. The role decision stays here,
// inside the gate's children function, so `AuthGate` imports neither of
// them and the companion's bundle keeps its distance from the editor.
createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <AuthGate>
      {(session) => (companion ? <DiagnosticsCompanion /> : <App session={session} />)}
    </AuthGate>
  </StrictMode>,
);
