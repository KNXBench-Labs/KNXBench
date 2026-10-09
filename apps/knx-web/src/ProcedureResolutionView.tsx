/** Displays ordered local procedure evidence separately from public sharing and execution. */
import type { ProcedureResolution } from "./procedureResolution";
import { useTranslate } from "./i18n";

export default function ProcedureResolutionView({ resolutions }: { resolutions: ProcedureResolution[] }) {
  const t = useTranslate();
  if (!resolutions.length) return null;
  return <section data-procedure-resolutions>
    <h3>{t("contribution.procedures.title")}</h3>
    <p className="contribution-warning">{t("contribution.procedures.boundary")}</p>
    <p>{t("contribution.procedures.privacy")}</p>
    {resolutions.map((r, index) => <details key={index} data-procedure-result>
      <summary>{r.programId} · {r.mask} · {r.style} · {r.variant} · {t(`contribution.procedures.${r.status}`)}</summary>
      <ul>{r.issues.map((i, n) => <li key={n}><strong>{i.code}</strong> · {i.location}</li>)}</ul>
      <details><summary>{t("contribution.procedures.sources")}</summary>
        <pre>{JSON.stringify({ sources: r.sources, programAttributes: r.programAttributes, templateAttributes: r.templateAttributes, merges: r.merges }, null, 2)}</pre>
      </details>
      {(r.omittedIssueCount ?? 0) > 0 && <p className="contribution-warning">{t("contribution.procedures.omitted", { count: r.omittedIssueCount ?? 0 })}</p>}
      {!!r.unplacedDeclarations?.length && <details data-procedure-unplaced><summary>{t("contribution.procedures.unplaced")}</summary><pre>{JSON.stringify(r.unplacedDeclarations, null, 2)}</pre></details>}
      <ol>{r.steps.map((s, n) => <li key={n} data-procedure-step>
        <code>{s.node.name}</code>
        <small>{s.origin.source} · {s.origin.location} · {s.node.byteStart}–{s.node.byteEnd}</small>
        <details><summary>{t("contribution.procedures.step")}</summary><pre>{JSON.stringify(s.node, null, 2)}</pre></details>
      </li>)}</ol>
    </details>)}
  </section>;
}
