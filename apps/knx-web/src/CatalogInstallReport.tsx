/** Install-report view and creation-diagnostic wording shared by catalog and device wizard. */
import type {
  CatalogInstallCount,
  CatalogInstallDiagnostic,
  CatalogInstallReport,
  CatalogUnknownConstruct,
  CreationDiagnostic,
} from "./api";
import { useTranslate } from "./i18n";
import type { MessageKey, Translate } from "./i18n";

// D4 exception (task-5 brief): `CreationDiagnostic.detail` is a ready-made
// English sentence composed server-side (`domain.rs`'s `.detail()`), but
// that means it can never be German. Instead of rendering it, this panel
// re-composes the sentence itself in the active UI language from the
// structured fields the server also sends — one catalogue key per `kind`.
// A `kind` this switch has never heard of (a future server variant an old
// frontend build doesn't know about yet) falls back to the server's raw
// `detail` string verbatim rather than rendering nothing — better an
// English sentence slips through once than a blank diagnostic line.
//
// The `!` below (task 5 review, minor): `CreationDiagnostic`'s TS interface
// is flat, so these fields are typed optional even though the server's
// tagged-enum serialization guarantees each is present for the `kind` that
// reads it — `catalogItemId` always accompanies `"programlessProduct"`,
// `refId` always accompanies `"ambiguousDpt"`/`"comObjectRefMissing"`, and
// so on. A `?? ""` fallback here was unreachable dead code, and worse, if
// it ever did fire (a future server bug lying about its own `kind`) it
// would render the sentence with a double space rather than surface the
// contradiction. A non-null assertion is the more honest way to say "this
// is guaranteed", loudly, at the one place it matters.
const categoryMessage: Record<CatalogInstallCount["category"], MessageKey> = {
  archive_member: "catalog.installReport.category.archiveMember",
  product: "catalog.installReport.category.product",
  application_program: "catalog.installReport.category.applicationProgram",
  parameter: "catalog.installReport.category.parameter",
  communication_object: "catalog.installReport.category.communicationObject",
  dynamic_node: "catalog.installReport.category.dynamicNode",
  module: "catalog.installReport.category.module",
  baggage_index: "catalog.installReport.category.baggageIndex",
  baggage: "catalog.installReport.category.baggage",
  unknown_construct: "catalog.installReport.category.unknownConstruct",
  master_section: "catalog.installReport.category.masterSection",
  master_subtree: "catalog.installReport.category.masterSubtree",
  datapoint_type: "catalog.installReport.category.datapointType",
};

function categoryLabel(t: Translate, category: CatalogInstallCount["category"]): string {
  return t(categoryMessage[category]);
}

const dispositionMessage: Record<CatalogInstallCount["disposition"], MessageKey> = {
  read: "catalog.installReport.disposition.read",
  stored: "catalog.installReport.disposition.stored",
  deduplicated: "catalog.installReport.disposition.deduplicated",
  "retained-but-uninterpreted": "catalog.installReport.disposition.retainedButUninterpreted",
  unsupported: "catalog.installReport.disposition.unsupported",
  dropped: "catalog.installReport.disposition.dropped",
};

function dispositionLabel(t: Translate, disposition: CatalogInstallCount["disposition"]): string {
  return t(dispositionMessage[disposition]);
}

const unknownKindMessage: Record<CatalogUnknownConstruct["kind"], MessageKey> = {
  Element: "catalog.installReport.unknownKind.element",
  Attribute: "catalog.installReport.unknownKind.attribute",
};

function unknownKindLabel(t: Translate, kind: CatalogUnknownConstruct["kind"]): string {
  return t(unknownKindMessage[kind]);
}

const diagnosticKindMessage: Record<CatalogInstallDiagnostic["kind"], MessageKey> = {
  "unsupported-master-section": "catalog.installReport.diagnosticKind.unsupportedMasterSection",
  "unsupported-master-subtree": "catalog.installReport.diagnosticKind.unsupportedMasterSubtree",
  "unresolved-baggage-declaration": "catalog.installReport.diagnosticKind.unresolvedBaggageDeclaration",
  "undeclared-baggage-payload": "catalog.installReport.diagnosticKind.undeclaredBaggagePayload",
};

const diagnosticDescriptionMessage: Record<CatalogInstallDiagnostic["kind"], MessageKey> = {
  "unsupported-master-section": "catalog.installReport.diagnostic.unsupportedMasterSection",
  "unsupported-master-subtree": "catalog.installReport.diagnostic.unsupportedMasterSubtree",
  "unresolved-baggage-declaration": "catalog.installReport.diagnostic.unresolvedBaggageDeclaration",
  "undeclared-baggage-payload": "catalog.installReport.diagnostic.undeclaredBaggagePayload",
};

function diagnosticLabel(t: Translate, kind: CatalogInstallDiagnostic["kind"]): string {
  return t(diagnosticKindMessage[kind]);
}

function describeInstallDiagnostic(t: Translate, diagnostic: CatalogInstallDiagnostic): string {
  const description = t(diagnosticDescriptionMessage[diagnostic.kind]);
  return t("catalog.installReport.diagnosticRow", {
    kind: diagnosticLabel(t, diagnostic.kind),
    detail: description,
    occurrences: diagnostic.occurrences,
    archivePath: diagnostic.archivePath,
    xmlPath: diagnostic.xmlPath,
  });
}
export function describeCreationDiagnostic(t: Translate, diagnostic: CreationDiagnostic): string {
  switch (diagnostic.kind) {
    case "programlessProduct":
      return t("catalogDiagnostic.programlessProduct", {
        catalogItemId: diagnostic.catalogItemId!,
      });
    case "ambiguousDpt":
      return t("catalogDiagnostic.ambiguousDpt", {
        refId: diagnostic.refId!,
        alternatives: (diagnostic.alternatives ?? []).join(", "),
      });
    case "comObjectRefMissing":
      return t("catalogDiagnostic.comObjectRefMissing", { refId: diagnostic.refId! });
    case "programRefMissing":
      return t("catalogDiagnostic.programRefMissing", { programRef: diagnostic.programRef! });
    case "dynamicOrModuleNotEvaluated":
      return t("catalogDiagnostic.dynamicOrModuleNotEvaluated", {
        programId: diagnostic.programId!,
      });
    default:
      return diagnostic.detail;
  }
}

/** One id per user submit (ADR-0069: 1–128 of `[A-Za-z0-9_-]`); a UUID fits. */
export function newRequestId(): string {
  if (typeof crypto.randomUUID === "function") return crypto.randomUUID();
  return Array.from(crypto.getRandomValues(new Uint8Array(16)), (b) => b.toString(16).padStart(2, "0")).join("");
}

/**
 * What a `.knxprod` install measured, exactly as the catalog has always
 * shown it: counts per category and disposition, unknown constructs,
 * diagnostics and the unverified-signature caution (KNOWN_LIMITATIONS §85).
 */
export function CatalogInstallReportView(props: { report: CatalogInstallReport }) {
  const t = useTranslate();
  const installReport = props.report;
  return (
    <>
      <section className="catalog-report" aria-live="polite" aria-label={t("catalog.installReport.heading")}>
          <p>
            {t("catalog.installReport.summary", {
              status: installReport.skipped
                ? t("catalog.installReport.status.already")
                : t("catalog.installReport.status.new"),
              scheme: installReport.scheme,
              members: t("catalog.installReport.membersCount", { count: installReport.members.length }),
              unknown: t("catalog.installReport.unknownCount", { count: installReport.unknown }),
              conflicts: t("catalog.installReport.conflictsCount", { count: installReport.conflicts }),
            })}
          </p>
          {installReport.facts ? (
            <>
              <h3>{t("catalog.installReport.factsMeasured")}</h3>
              <h4>{t("catalog.installReport.countsHeading")}</h4>
              <ul>
                {installReport.facts.counts.map((row) => (
                  <li key={`${row.category}:${row.disposition}`}>
                    {t("catalog.installReport.countRow", {
                      category: categoryLabel(t, row.category),
                      disposition: dispositionLabel(t, row.disposition),
                      count: row.count,
                    })}
                  </li>
                ))}
              </ul>
              <h4>{t("catalog.installReport.unknownHeading")}</h4>
              <p>
                {t("catalog.installReport.unknownSummary", {
                  distinct: installReport.facts.unknownConstructs.length,
                  occurrences: installReport.facts.unknownOccurrences,
                })}
              </p>
              {installReport.facts.unknownConstructs.length > 0 && (
                <ul>
                  {installReport.facts.unknownConstructs.map((unknown) => (
                    <li key={`${unknown.xpath}:${unknown.kind}:${unknown.name}`}>
                      {t("catalog.installReport.unknownRow", {
                        kind: unknownKindLabel(t, unknown.kind),
                        name: unknown.name,
                        xpath: unknown.xpath,
                        occurrences: unknown.occurrences,
                      })}
                      {unknown.sample ? ` — ${unknown.sample}` : ""}
                    </li>
                  ))}
                </ul>
              )}
              {installReport.facts.diagnostics.length > 0 && (
                <>
                  <h4>{t("catalog.installReport.diagnosticsHeading")}</h4>
                  <ul>
                    {installReport.facts.diagnostics.map((diagnostic) => (
                      <li key={`${diagnostic.kind}:${diagnostic.archivePath}:${diagnostic.xmlPath}`}>
                        {describeInstallDiagnostic(t, diagnostic)}
                      </li>
                    ))}
                  </ul>
                </>
              )}
            </>
          ) : (
            <p>{t("catalog.installReport.factsUnavailable")}</p>
          )}
      </section>
      {/* KNOWN_LIMITATIONS.md §85: a `.signature` member's `role` starts
          with "Signature" whether or not it was checked — because nothing
          ever checks it. This is the one place a person reads an install
          report; the count exists so "installed cleanly" and "signed and
          verified" cannot be the same sentence in either language. */}
      {installReport.members.some((member) => member.role.startsWith("Signature")) && (
        <p className="catalog-report catalog-report-caution">
          {t("catalog.installReport.unverifiedSignature", {
            count: installReport.members.filter((member) => member.role.startsWith("Signature")).length,
          })}
        </p>
      )}
    </>
  );
}
