/** Modal add-device wizard: product, placement, naming, server preview, confirmed create. */
import { useEffect, useRef, useState } from "react";
import * as api from "./api";
import type { CatalogItem, CatalogPreview, CreatedCatalogDevice, CreationDiagnostic } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { describeCreationDiagnostic, newRequestId } from "./CatalogInstallReport";
import DeviceWizardProduct from "./DeviceWizardProduct";
import {
  buildingPartChoices,
  initialPlacement,
  lineChoices,
  withInstallation,
  type DeviceWizardTarget,
  type Placement,
} from "./deviceWizardPlacement";
import { useTranslate, type MessageKey } from "./i18n";
import Overlay from "./Overlay";

type Step = "product" | "placement" | "naming" | "review" | "result";
const STEPS: readonly Step[] = ["product", "placement", "naming", "review", "result"];
const STEP_LABEL: Record<Step, MessageKey> = {
  product: "deviceWizard.step.product",
  placement: "deviceWizard.step.placement",
  naming: "deviceWizard.step.naming",
  review: "deviceWizard.step.review",
  result: "deviceWizard.step.result",
};
/** The catalog's own batch bound (`MAX_CATALOG_QUANTITY` on the server). */
const MAX_QUANTITY = 32;

type PreviewState =
  | { status: "loading" }
  | { status: "ready"; preview: CatalogPreview }
  | { status: "failed"; message: string };

/**
 * ADR-0093 §5. A guided path beside the catalog workspace, over the same
 * routes: `POST /api/devices/preview` computes on a copy of the project
 * what a create would do now, and the create sends those names and
 * addresses back as `expected`, so a project that changed in between is
 * refused (`409`) and previewed again instead of creating something the
 * user never saw. Parameters and group links stay in their own views.
 */
export default function DeviceWizard(props: {
  tree: ProjectTree;
  target: DeviceWizardTarget;
  /** Skips the product step when the catalog already chose one. */
  product?: CatalogItem;
  onCreated: (tree: ProjectTree) => void;
  onOpenDevice: (deviceId: number) => void;
  onClose: () => void;
}) {
  const { tree, target, onCreated, onOpenDevice, onClose } = props;
  const t = useTranslate();
  const [step, setStep] = useState<Step>(props.product ? "placement" : "product");
  const [product, setProduct] = useState<CatalogItem | null>(props.product ?? null);
  const [placement, setPlacement] = useState<Placement>(() => initialPlacement(tree, target));
  const [name, setName] = useState(props.product?.name ?? "");
  const [quantity, setQuantity] = useState("1");
  const [allocate, setAllocate] = useState(false);
  const [uniqueNames, setUniqueNames] = useState(false);
  const [preview, setPreview] = useState<PreviewState | null>(null);
  const [previewVersion, setPreviewVersion] = useState(0);
  const [staleNotice, setStaleNotice] = useState(false);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [created, setCreated] = useState<{ items: CreatedCatalogDevice[]; diagnostics: CreationDiagnostic[] } | null>(null);
  // A lost or 5xx response: the outcome is unknown. Resending the same
  // request id is safe only while the same server process holds it (ADR-0069).
  const [unconfirmed, setUnconfirmed] = useState<{ requestId: string; incarnation: string | undefined } | null>(null);
  const [installedPackage, setInstalledPackage] = useState(false);
  const [touched, setTouched] = useState(Boolean(props.product));
  const [confirmDiscard, setConfirmDiscard] = useState(false);
  const previewRequestRef = useRef(0);
  const headingRef = useRef<HTMLHeadingElement>(null);

  const installation = tree.installations.find((i) => i.id === placement.installationId);
  const lines = lineChoices(installation);
  const parts = buildingPartChoices(installation);
  const quantityNumber = Number(quantity);
  const quantityValid = /^\d+$/.test(quantity.trim()) && quantityNumber >= 1 && quantityNumber <= MAX_QUANTITY;
  const namingValid = name.trim() !== "" && quantityValid;
  const allocateEffective = allocate && placement.lineId !== null;
  const stepIndex = STEPS.indexOf(step);
  const reachable = (which: Step) => which === "product"
    || (product !== null && (which === "placement" || (which === "naming") || (which === "review" && namingValid)))
    || (which === "result" && created !== null);

  useEffect(() => {
    headingRef.current?.focus();
  }, [step]);

  // The preview is asked for whenever the review step shows inputs it has
  // not computed yet; an answer to an older question is dropped.
  useEffect(() => {
    if (step !== "review" || !product || !namingValid) return;
    const id = ++previewRequestRef.current;
    setPreview({ status: "loading" });
    api.previewDevices({
      lineId: placement.lineId, catalogItemId: product.id, name: name.trim(), quantity: quantityNumber,
      allocateAddresses: allocateEffective, uniqueNames,
      ...(placement.installationId === null ? {} : { installationId: placement.installationId }),
      ...(placement.buildingPartId === null ? {} : { buildingPartId: placement.buildingPartId }),
    }).then((answer) => {
      if (id === previewRequestRef.current) setPreview({ status: "ready", preview: answer });
    }).catch((e) => {
      if (id === previewRequestRef.current) setPreview({ status: "failed", message: api.errorMessage(e) });
    });
  }, [step, product, placement, name, quantityNumber, allocateEffective, uniqueNames, namingValid, previewVersion]);

  function edit<T>(setter: (value: T) => void): (value: T) => void {
    return (value) => {
      setTouched(true);
      setConfirmDiscard(false);
      setter(value);
    };
  }

  function close() {
    onClose();
  }

  // Escape or backdrop: ask first once something was chosen; a second Escape
  // keeps editing. After a create there is nothing left to lose.
  function dismiss() {
    if (created || unconfirmed || !touched) close();
    else setConfirmDiscard((showing) => !showing);
  }

  async function create(resend?: { requestId: string; incarnation: string | undefined }) {
    if (!product || preview?.status !== "ready" || creating) return;
    const requestId = resend?.requestId ?? newRequestId();
    const incarnation = resend?.incarnation ?? tree.server_incarnation;
    setCreating(true);
    setError(null);
    setStaleNotice(false);
    try {
      if (resend) {
        const current = (await api.currentProject()).server_incarnation;
        if (current !== resend.incarnation) {
          setUnconfirmed(null);
          setError(t("catalog.retryServerRestarted"));
          return;
        }
      }
      const response = await api.createDevice(
        placement.lineId, product.id, name.trim(), quantityNumber, requestId,
        {
          allocateAddresses: allocateEffective, uniqueNames,
          ...(placement.installationId === null ? {} : { installationId: placement.installationId }),
          ...(placement.buildingPartId === null ? {} : { buildingPartId: placement.buildingPartId }),
          expected: preview.preview.items.map((item) => ({ name: item.name, address: item.address })),
        },
      );
      setUnconfirmed(null);
      onCreated(response.tree);
      setCreated({ items: response.items ?? [], diagnostics: response.diagnostics });
      setStep("result");
    } catch (e) {
      const status = (e as { status?: unknown } | null)?.status;
      if (api.isPreviewStale(e)) {
        setStaleNotice(true);
        setPreviewVersion((v) => v + 1);
      } else if (typeof status !== "number" || status >= 500) {
        setUnconfirmed({ requestId, incarnation });
        setError(`${api.errorMessage(e)} ${t("catalog.unconfirmedBatch")}`);
      } else {
        setError(api.errorMessage(e));
      }
    } finally {
      setCreating(false);
    }
  }

  function addMore() {
    setCreated(null);
    setPreview(null);
    setError(null);
    setStep("naming");
  }

  const lineLabel = lines.find((l) => l.id === placement.lineId)?.label;
  const partLabel = parts.find((p) => p.id === placement.buildingPartId)?.label;

  return (
    <Overlay labelledBy="device-wizard-title" className="new-project-panel project-wizard device-wizard" onClose={dismiss}
      initialFocusRef={headingRef}>
      <h2 className="settings-panel-title" id="device-wizard-title">{t("deviceWizard.title")}</h2>
      <ol className="project-wizard-steps" aria-label={t("projectWizard.stepsLabel")}>
        {STEPS.map((which, index) => (
          <li key={which}>
            <button type="button" className="project-wizard-step-link" aria-current={which === step ? "step" : undefined}
              disabled={!reachable(which) || created !== null && which !== "result" || unconfirmed !== null}
              onClick={() => setStep(which)}>
              <span className="project-wizard-step-number" aria-hidden="true">{index + 1}</span>
              {t(STEP_LABEL[which])}
            </button>
          </li>
        ))}
      </ol>
      <h3 className="project-wizard-step-title" ref={headingRef} tabIndex={-1}>
        {t("projectWizard.stepOf", { current: stepIndex + 1, total: STEPS.length, step: t(STEP_LABEL[step]) })}
      </h3>
      {product && step !== "product" && (
        <p className="device-wizard-product">{t("deviceWizard.selectedProduct", { name: product.name ?? product.id })}</p>
      )}

      {step === "product" && (
        <DeviceWizardProduct selected={product} t={t} onInstalled={() => { setTouched(true); setInstalledPackage(true); }}
          onSelect={(item) => {
            edit(setProduct)(item);
            if (name.trim() === "" || name === product?.name) setName(item.name ?? "");
          }} />
      )}

      {step === "placement" && (
        <div className="device-wizard-step">
          <p className="project-wizard-intro">{t("deviceWizard.placement.intro")}</p>
          {tree.installations.length > 1 && (
            <label className="settings-field">
              <span className="settings-field-label">{t("deviceWizard.placement.installation")}</span>
              <select value={placement.installationId ?? ""}
                onChange={(e) => edit(setPlacement)(withInstallation(tree, placement, Number(e.target.value)))}>
                {tree.installations.map((i) => <option key={i.id} value={i.id}>{i.name || `#${i.id}`}</option>)}
              </select>
            </label>
          )}
          <label className="settings-field">
            <span className="settings-field-label">{t("deviceWizard.placement.line")}</span>
            <select value={placement.lineId ?? ""}
              onChange={(e) => edit(setPlacement)({ ...placement, lineId: e.target.value === "" ? null : Number(e.target.value) })}>
              <option value="">{t("deviceWizard.placement.noLine")}</option>
              {lines.map((line) => <option key={line.id} value={line.id}>{line.label}</option>)}
            </select>
          </label>
          <label className="settings-field">
            <span className="settings-field-label">{t("deviceWizard.placement.buildingPart")}</span>
            <select value={placement.buildingPartId ?? ""}
              onChange={(e) => edit(setPlacement)({ ...placement, buildingPartId: e.target.value === "" ? null : Number(e.target.value) })}>
              <option value="">{t("deviceWizard.placement.noBuildingPart")}</option>
              {parts.map((part) => <option key={part.id} value={part.id}>{part.label}</option>)}
            </select>
          </label>
          {lines.length === 0 && parts.length === 0 && (
            <p className="settings-field-hint">{t("deviceWizard.placement.nothingToChoose")}</p>
          )}
        </div>
      )}

      {step === "naming" && (
        <div className="device-wizard-step">
          <label className="settings-field">
            <span className="settings-field-label">{t("deviceWizard.naming.name")}</span>
            <input value={name} aria-invalid={name.trim() === ""} onChange={(e) => edit(setName)(e.target.value)} />
          </label>
          <label className="settings-field">
            <span className="settings-field-label">{t("catalog.quantity")}</span>
            <input value={quantity} inputMode="numeric" aria-invalid={!quantityValid}
              onChange={(e) => edit(setQuantity)(e.target.value)} />
            {!quantityValid && <span className="field-error">{t("catalog.quantityInvalid")}</span>}
          </label>
          <label className="device-wizard-check">
            <input type="checkbox" checked={allocateEffective} disabled={placement.lineId === null}
              onChange={(e) => edit(setAllocate)(e.target.checked)} />
            {t("catalog.allocateAddresses")}
          </label>
          {placement.lineId === null && <p className="settings-field-hint">{t("catalog.allocateNeedsLine")}</p>}
          <label className="device-wizard-check">
            <input type="checkbox" checked={uniqueNames} onChange={(e) => edit(setUniqueNames)(e.target.checked)} />
            {t("catalog.uniqueNames")}
          </label>
        </div>
      )}

      {step === "review" && (
        <div className="device-wizard-step" aria-live="polite">
          <dl className="project-wizard-summary">
            <dt>{t("deviceWizard.placement.line")}</dt><dd>{lineLabel ?? t("deviceWizard.placement.noLine")}</dd>
            <dt>{t("deviceWizard.placement.buildingPart")}</dt><dd>{partLabel ?? t("deviceWizard.placement.noBuildingPart")}</dd>
            {tree.installations.length > 1 && (<>
              <dt>{t("deviceWizard.placement.installation")}</dt><dd>{installation?.name ?? ""}</dd>
            </>)}
          </dl>
          {staleNotice && <p className="field-error" role="alert">{t("deviceWizard.review.stale")}</p>}
          {preview?.status === "loading" && <p>{t("deviceWizard.review.loading")}</p>}
          {preview?.status === "failed" && <p className="field-error" role="alert">{preview.message}</p>}
          {preview?.status === "ready" && (
            <>
              <h4>{t("deviceWizard.review.devices", { count: preview.preview.items.length })}</h4>
              <ul className="device-wizard-preview">
                {preview.preview.items.map((item) => (
                  <li key={item.index}>
                    <strong>{item.name}</strong>{" "}
                    <span className="device-wizard-result-meta">
                      {item.address ?? t("deviceWizard.review.noAddress")}
                    </span>
                  </li>
                ))}
              </ul>
              {preview.preview.diagnostics.length > 0 && (
                <ul className="device-wizard-diagnostics">
                  {[...new Set(preview.preview.diagnostics.map((d) => describeCreationDiagnostic(t, d)))].map((text) => (
                    <li key={text}>{text}</li>
                  ))}
                </ul>
              )}
              <p className="settings-field-hint">{t("deviceWizard.review.hint")}</p>
            </>
          )}
        </div>
      )}

      {step === "result" && created && (
        <div className="device-wizard-step" role="status">
          <p>{t("deviceWizard.result.created", { count: created.items.length })}</p>
          <ul className="device-wizard-preview">
            {created.items.map((item) => (
              <li key={item.deviceId}>
                <strong>{item.name}</strong>
                {item.address && <span className="device-wizard-result-meta"> {item.address}</span>}
                {item.diagnostics.length > 0 && (
                  <ul className="device-wizard-diagnostics">
                    {item.diagnostics.map((d, i) => <li key={`${d.kind}-${i}`}>{describeCreationDiagnostic(t, d)}</li>)}
                  </ul>
                )}
              </li>
            ))}
          </ul>
          <p className="settings-field-hint">{t("deviceWizard.result.next")}</p>
        </div>
      )}

      {confirmDiscard && (
        <section className="project-wizard-discard" role="alert">
          <p><strong>{t("projectWizard.discard.title")}</strong> {t("deviceWizard.discard.body")}</p>
          {installedPackage && <p>{t("deviceWizard.discard.packageStays")}</p>}
          <div className="new-project-actions">
            <button type="button" onClick={() => setConfirmDiscard(false)}>{t("projectWizard.discard.keep")}</button>
            <button type="button" onClick={close}>{t("projectWizard.discard.confirm")}</button>
          </div>
        </section>
      )}

      {error && <p className="field-error" role="alert">{error}</p>}

      <div className="new-project-actions">
        {step === "result" ? (
          <>
            <button type="button" onClick={addMore}>{t("deviceWizard.result.addMore")}</button>
            {created && created.items.length > 0 && (
              <button type="button" onClick={() => { onOpenDevice(created.items[0].deviceId); close(); }}>
                {t("deviceWizard.result.open")}
              </button>
            )}
            <button type="button" className="primary-action" onClick={close}>{t("projectWizard.done.close")}</button>
          </>
        ) : unconfirmed ? (
          <>
            <button type="button" disabled={creating} onClick={() => void create(unconfirmed)}>{t("catalog.retrySafely")}</button>
            <button type="button" onClick={close}>{t("catalog.done")}</button>
          </>
        ) : (
          <>
            <button type="button" onClick={() => (touched ? setConfirmDiscard(true) : close())}>{t("newProject.cancel")}</button>
            <button type="button" disabled={stepIndex === 0} onClick={() => setStep(STEPS[stepIndex - 1])}>
              {t("projectWizard.back")}
            </button>
            {step !== "review" ? (
              <button type="button" className="primary-action" disabled={!reachable(STEPS[stepIndex + 1])}
                onClick={() => setStep(STEPS[stepIndex + 1])}>
                {t("projectWizard.next")}
              </button>
            ) : (
              <button type="button" className="primary-action" disabled={preview?.status !== "ready" || creating}
                onClick={() => void create()}>
                {creating ? t("catalog.creating") : t("deviceWizard.review.create", { count: quantityValid ? quantityNumber : 1 })}
              </button>
            )}
          </>
        )}
      </div>
    </Overlay>
  );
}
