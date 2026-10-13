/** "Rename…" dialog for an explicit group-address selection (Goal B7). */
/*
 * Literal prefix and suffix only — the existing FUTURE-02 service decides
 * (`docs/contracts/offline-name-recipe.md`). The preview is bound to the
 * tree the user is looking at; editing the text or any change to the
 * project retires it, and only the shown preview can be confirmed. One
 * confirmation is one undo step. No pattern, no generated selection, no bus.
 */
import { useEffect, useRef, useState } from "react";
import type { ProjectTree } from "../../bindings/ProjectTree";
import Overlay from "../../Overlay";
import { useTranslate } from "../../i18n";
import { useGroupAddressFormat } from "../../gaNotation";
import { findGroupAddress } from "../../treeUtils";
import {
  applyNameRecipe, cancelNameRecipe, isStaleRecipe, MAX_RECIPE_AFFIX_BYTES, MAX_RECIPE_SELECTION,
  previewNameRecipe, utf8Bytes, type RecipeBinding, type RecipePreview, type RecipeRefusal,
} from "./client";
import "./nameRecipe.css";

const REFUSALS = {
  nameRecipeSelection: "nameRecipe.refusal.selection",
  nameRecipeBudget: "nameRecipe.refusal.budget",
  nameRecipeDuplicate: "nameRecipe.refusal.duplicate",
  nameRecipeTarget: "nameRecipe.refusal.target",
  nameRecipeInvalidName: "nameRecipe.refusal.invalidName",
  nameRecipeRefused: "nameRecipe.refusal.refused",
  nameRecipePersistenceFailed: "nameRecipe.refusal.persistence",
} as const;

type Problem = { lead: string; detail: string | null };

export function recipeBinding(tree: ProjectTree): RecipeBinding | null {
  const { server_incarnation, project_incarnation, snapshot_revision } = tree;
  if (server_incarnation == null || project_incarnation == null || snapshot_revision == null) return null;
  return { serverIncarnation: server_incarnation, projectIncarnation: project_incarnation, snapshotRevision: snapshot_revision };
}

const sameBinding = (a: RecipeBinding | null, b: RecipeBinding | null) => a !== null && b !== null
  && a.serverIncarnation === b.serverIncarnation && a.projectIncarnation === b.projectIncarnation
  && a.snapshotRevision === b.snapshotRevision;

export default function NameRecipeDialog(props: {
  tree: ProjectTree;
  /** Stable ids in the order the user selected them. */
  ids: readonly number[];
  /** After a confirmed rename: the caller re-reads the project. */
  onApplied: () => void;
  /** The server says the shown tree is old: the caller re-reads it, so the next preview can bind. */
  onStale: () => void;
  onClose: () => void;
}) {
  const t = useTranslate();
  const format = useGroupAddressFormat();
  const [prefix, setPrefix] = useState("");
  const [suffix, setSuffix] = useState("");
  const [preview, setPreview] = useState<{ plan: RecipePreview; binding: RecipeBinding } | null>(null);
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<Problem | null>(null);
  const tokenRef = useRef<string | null>(null);
  const binding = recipeBinding(props.tree);
  const tooMany = props.ids.length > MAX_RECIPE_SELECTION;
  const tooLong = utf8Bytes(prefix) > MAX_RECIPE_AFFIX_BYTES || utf8Bytes(suffix) > MAX_RECIPE_AFFIX_BYTES;
  const empty = prefix === "" && suffix === "";

  function retire() {
    if (tokenRef.current !== null) cancelNameRecipe(tokenRef.current);
    tokenRef.current = null;
    setPreview(null);
  }
  // A preview belongs to one snapshot: any project change retires it.
  useEffect(() => {
    if (preview && !sameBinding(preview.binding, binding)) {
      retire();
      setProblem({ lead: t("nameRecipe.stale"), detail: null });
    }
  }, [binding?.serverIncarnation, binding?.projectIncarnation, binding?.snapshotRevision]);
  // Closing the dialog forgets the preview on the server too.
  useEffect(() => () => { if (tokenRef.current !== null) cancelNameRecipe(tokenRef.current); }, []);

  function explain(error: unknown): Problem {
    if (isStaleRecipe(error)) {
      props.onStale();
      return { lead: t("nameRecipe.stale"), detail: null };
    }
    const refusal = error as Partial<RecipeRefusal>;
    const key = typeof refusal.kind === "string" && Object.hasOwn(REFUSALS, refusal.kind)
      ? REFUSALS[refusal.kind as keyof typeof REFUSALS] : "nameRecipe.failed";
    return { lead: t(key),
      detail: error instanceof Error ? error.message : String(error) };
  }

  async function showPreview() {
    if (!binding || busy || tooMany || tooLong) return;
    retire();
    setBusy(true); setProblem(null);
    try {
      const plan = await previewNameRecipe(binding, props.ids, prefix, suffix);
      tokenRef.current = plan.token;
      setPreview({ plan, binding });
    } catch (error) {
      setProblem(explain(error));
    } finally {
      setBusy(false);
    }
  }

  async function confirm() {
    if (!preview || busy || preview.plan.noOp) return;
    setBusy(true); setProblem(null);
    try {
      await applyNameRecipe(preview.plan.token);
      tokenRef.current = null;
      props.onApplied();
      props.onClose();
    } catch (error) {
      // Stale or refused: nothing was renamed. The old preview is spent.
      tokenRef.current = null;
      setPreview(null);
      setProblem(explain(error));
    } finally {
      setBusy(false);
    }
  }

  function edit(setter: (value: string) => void, value: string) {
    setter(value);
    if (preview) retire();
  }

  const count = props.ids.length;
  return <Overlay className="name-recipe-panel" labelledBy="name-recipe-title" onClose={busy ? () => {} : props.onClose}>
    <header className="name-recipe-heading">
      <h2 id="name-recipe-title">{t("nameRecipe.title", { count })}</h2>
      <p>{t("nameRecipe.subtitle")}</p>
    </header>
    {tooMany && <p className="field-error" role="alert">{t("nameRecipe.tooMany", { max: MAX_RECIPE_SELECTION })}</p>}
    <form className="name-recipe-form" onSubmit={(event) => { event.preventDefault(); void showPreview(); }}>
      <label>{t("nameRecipe.prefix")}
        <input value={prefix} disabled={busy} data-recipe-prefix onInput={(event) => edit(setPrefix, event.currentTarget.value)} />
      </label>
      <label>{t("nameRecipe.suffix")}
        <input value={suffix} disabled={busy} data-recipe-suffix onInput={(event) => edit(setSuffix, event.currentTarget.value)} />
      </label>
      {tooLong && <p className="field-error">{t("nameRecipe.tooLong")}</p>}
      <p className="name-recipe-hint">{t("nameRecipe.literalHint")}</p>
      <button type="submit" disabled={busy || !binding || tooMany || tooLong || empty}>{t("nameRecipe.preview")}</button>
    </form>
    {problem && <div className="name-recipe-problem" role="alert"><strong>{problem.lead}</strong>
      {problem.detail && <><p>{problem.detail}</p><small>{t("toast.error.messageIsEnglish")}</small></>}</div>}
    {preview && <section className="name-recipe-preview" aria-labelledby="name-recipe-preview-title">
      <h3 id="name-recipe-preview-title">{preview.plan.noOp ? t("nameRecipe.nothingChanges")
        : t("nameRecipe.changes", { count: preview.plan.changed, total: count })}</h3>
      <div className="name-recipe-table-wrap">
        <table className="name-recipe-table">
          <thead><tr><th scope="col">{t("nameRecipe.address")}</th><th scope="col">{t("nameRecipe.before")}</th>
            <th scope="col">{t("nameRecipe.after")}</th></tr></thead>
          <tbody>{preview.plan.rows.map((row) => {
            const node = findGroupAddress(props.tree, row.id);
            return <tr key={row.id} data-recipe-row={row.id} data-changed={row.changed}>
              <td>{node ? format(node.address) : `#${row.id}`}</td>
              <td>{row.before}</td>
              <td>{row.changed ? row.after : <span className="name-recipe-unchanged">{t("nameRecipe.unchanged")}</span>}</td>
            </tr>;
          })}</tbody>
        </table>
      </div>
      <p className="name-recipe-undo">{t("nameRecipe.undoHint")}</p>
    </section>}
    <footer className="name-recipe-actions">
      <button type="button" disabled={busy} onClick={props.onClose}>{t("nameRecipe.cancel")}</button>
      <button type="button" className="primary-action" data-recipe-confirm disabled={busy || !preview || preview.plan.noOp}
        onClick={() => void confirm()}>
        {preview && !preview.plan.noOp ? t("nameRecipe.confirm", { count: preview.plan.changed }) : t("nameRecipe.confirmIdle")}</button>
    </footer>
  </Overlay>;
}
