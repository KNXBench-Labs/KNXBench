/** Inline list of the wizard's structure issues that belong to one node or step. */
import type { Translate } from "./i18n";
import type { StructureIssue } from "./projectSeed";

export function IssueList(props: { issues: readonly StructureIssue[]; nodeKey: string | null; t: Translate }) {
  const own = props.issues.filter((issue) => issue.nodeKey === props.nodeKey);
  if (own.length === 0) return null;
  return (
    <ul className="project-wizard-issues">
      {own.map((issue, index) => (
        <li key={`${issue.message}:${index}`} className="field-error">{props.t(issue.message, issue.params)}</li>
      ))}
    </ul>
  );
}
