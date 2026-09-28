/** File-menu button that opens the project documentation preview and export dialog. */
import { useEffect, useRef, useState } from "react";
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
  const buttonRef = useRef<HTMLButtonElement | null>(null);

  // `Overlay` gives focus back to this button on close, but the File menu
  // around it closed when the entry was chosen, so the button is hidden and
  // focus would fall to `<body>`. Moving it to the menu's `<summary>` (the
  // nearest visible control) happens after the overlay's own restoration,
  // hence in an effect keyed on `open` rather than in the close handler.
  const wasOpen = useRef(false);
  useEffect(() => {
    if (wasOpen.current && !open) {
      buttonRef.current?.closest("details")?.querySelector("summary")?.focus();
    }
    wasOpen.current = open;
  }, [open]);

  return (
    <>
      <button ref={buttonRef} onClick={() => setOpen(true)} disabled={!tree}>
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
