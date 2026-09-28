/** File-menu button that opens the project documentation preview and export dialog. */
import { useState } from "react";
import type { ProjectTree } from "./bindings/ProjectTree";
import DocumentationDialog from "./DocumentationDialog";
import { useTranslate } from "./i18n";

// Opens `DocumentationDialog`, which previews and writes the live project as
// one self-contained "project documentation" HTML file (`crates/knx-report`,
// `POST /api/project/documentation-preview` and `-export`). Never labelled
// an "ETS report" — no ETS-produced sample exists anywhere in this
// repository to be compatible with.
//
// Same precedent as `GroupAddressCsvButtons.tsx`: the dialog owns its
// `./api`/`./filePicker` calls outright, with the toast plumbing it does
// need coming in as narrow callbacks from `App.tsx`. This export never
// touches the open project's tree, so there is no `onTreeUpdate`.
export default function DocumentationExportButton(props: {
  tree: ProjectTree | null;
  onSummary: (message: string) => void;
  onError: (e: unknown) => void;
  onClearErrors: () => void;
}) {
  const { tree, onSummary, onError, onClearErrors } = props;
  const t = useTranslate();
  const [open, setOpen] = useState(false);

  return (
    <>
      <button onClick={() => setOpen(true)} disabled={!tree}>
        {t("documentationExport.button")}
      </button>
      {open && (
        <DocumentationDialog
          onSummary={onSummary}
          onError={onError}
          onClearErrors={onClearErrors}
          onClose={() => setOpen(false)}
        />
      )}
    </>
  );
}
