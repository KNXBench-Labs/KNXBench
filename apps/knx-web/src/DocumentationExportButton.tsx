import { pickSavePath } from "./filePicker";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";

// Writes the live project as one self-contained "project documentation"
// HTML file (`crates/knx-report`, `POST /api/project/documentation-export`).
// Never labelled an "ETS report" — no ETS-produced sample exists anywhere
// in this repository to be compatible with.
//
// Same precedent as `GroupAddressCsvButtons.tsx`: a standalone component
// that owns its `./api`/`./filePicker` calls outright rather than taking
// them as props, with the toast/tree plumbing it does need coming in as
// narrow callbacks from `App.tsx`, which already owns both. This export
// never touches the open project's tree, so unlike `GroupAddressCsvButtons`
// there is no `onTreeUpdate`.
const DOCUMENTATION_FILTER = [{ name: "HTML document", extensions: ["html"] }];

export default function DocumentationExportButton(props: {
  tree: ProjectTree | null;
  onSummary: (message: string) => void;
  onError: (e: unknown) => void;
  onClearErrors: () => void;
}) {
  const { tree, onSummary, onError, onClearErrors } = props;

  async function exportDocumentation() {
    const path = await pickSavePath(DOCUMENTATION_FILTER, "project-documentation.html");
    if (!path) return;
    // Sequenced exactly like `GroupAddressCsvButtons`'s `exportCsv`: clear
    // any leftover error toast before this operation runs, not after — so
    // a stale error never sits on screen through a subsequent success.
    onClearErrors();
    try {
      const { warnings } = await api.exportDocumentation(path);
      const n = warnings.length;
      onSummary(
        n === 0
          ? "Project documentation exported, no warnings."
          : `Project documentation exported, ${n} warning${n === 1 ? "" : "s"} — see Log.`,
      );
    } catch (e) {
      onError(e);
    }
  }

  return (
    <button onClick={exportDocumentation} disabled={!tree}>
      Export documentation…
    </button>
  );
}
