/** Projects the closed admission vocabulary without freezing translated sentences. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { useTranslate, type MessageKey } from "./i18n";
import type { ThemePackFileDiagnostic } from "./themePackFiles";

const LABELS: Record<ThemePackFileDiagnostic["kind"], MessageKey> = {
  invalidJson: "themePack.diagnostic.invalidJson",
  duplicateKey: "themePack.diagnostic.duplicateKey",
  sizeLimit: "themePack.diagnostic.sizeLimit",
  depthLimit: "themePack.diagnostic.depthLimit",
  nodeLimit: "themePack.diagnostic.nodeLimit",
  invalidContract: "themePack.diagnostic.invalidContract",
  unsupportedFormat: "themePack.diagnostic.unsupportedFormat",
  unsupportedVersion: "themePack.manager.unsupportedVersion",
  invalidMetadata: "themePack.diagnostic.invalidMetadata",
  invalidTokens: "themePack.diagnostic.invalidTokens",
  invalidValue: "themePack.diagnostic.invalidValue",
  invalidAccents: "themePack.diagnostic.invalidAccents",
  contrast: "themePack.diagnostic.contrast",
  identityMismatch: "themePack.diagnostic.identityMismatch",
  storeLimit: "themePack.diagnostic.storeLimit",
  missingSelection: "themePack.diagnostic.missingSelection",
  fileRead: "themePack.diagnostic.fileRead",
  invalidEncoding: "themePack.diagnostic.invalidEncoding",
};

export default function ThemePackDiagnostic({ diagnostic }: { diagnostic: ThemePackFileDiagnostic }) {
  const t = useTranslate();
  return <div data-theme-diagnostic={diagnostic.kind}>
    <p>{t(LABELS[diagnostic.kind])}</p>
    <code>{diagnostic.kind}</code>{" "}<code>{diagnostic.path}</code>
  </div>;
}
