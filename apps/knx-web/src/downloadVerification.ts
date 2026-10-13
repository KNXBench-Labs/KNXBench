/** Tester verification (ADR-0121): plan warnings by rule, and a report's checklist. */
// Pure: the panel renders what these return. Warnings arrive from the server
// as `rule: detail (reference)`; a known rule gets a plain translated text,
// an unknown one is shown as sent, never dropped.

import type { MessageKey } from "./i18n";

/** Rules this build explains in plain language (ADR-0120). */
const KNOWN_RULES: Record<string, MessageKey> = {
  "base-image-low-octet-first": "deviceDownload.warning.baseImageLowOctetFirst",
  "base-image-order-unchecked": "deviceDownload.warning.baseImageOrderUnchecked",
};

/** One plan warning, split for display. */
export interface PlanWarning {
  /** The stable rule name, e.g. `base-image-low-octet-first`. */
  rule: string;
  /** The plain explanation, when this build knows the rule. */
  explanation: MessageKey | null;
  /** The server's full text, kept for the details. */
  text: string;
}

/** Splits `rule: detail (reference)` warnings; a text without a rule keeps it whole. */
export function planWarnings(warnings: readonly string[]): PlanWarning[] {
  return warnings.map((text) => {
    const colon = text.indexOf(":");
    const rule = colon > 0 ? text.slice(0, colon).trim() : text.trim();
    return { rule, explanation: KNOWN_RULES[rule] ?? null, text };
  });
}

/** The fields of a report the checklist reads. */
export interface ChecklistInput {
  scope: string;
  warnings: readonly string[];
  plan: { groupLinks: number };
  outcome: { completed: boolean; written: "yes" | "no" | "partially"; restart: string | null } | null;
}

export type CheckId = "groupAddresses" | "status" | "parameters" | "multiOctet" | "restart";

const CHECK_KEYS: Record<CheckId, MessageKey> = {
  groupAddresses: "deviceDownload.verify.check.groupAddresses",
  status: "deviceDownload.verify.check.status",
  parameters: "deviceDownload.verify.check.parameters",
  multiOctet: "deviceDownload.verify.check.multiOctet",
  restart: "deviceDownload.verify.check.restart",
};

export function checkKey(id: CheckId): MessageKey {
  return CHECK_KEYS[id];
}

/** Whether the run reached its end with every block written: only then is there a device to check. */
export function runCompleted(input: ChecklistInput): boolean {
  return input.outcome !== null && input.outcome.completed && input.outcome.written === "yes";
}

/**
 * What a tester should check on the device after this download. Empty for a
 * run that did not complete: there is nothing to check, only a failure to
 * report.
 */
export function checklist(input: ChecklistInput): CheckId[] {
  if (!runCompleted(input)) return [];
  const checks: CheckId[] = [];
  if (input.outcome?.restart === "unconfirmed") checks.push("restart");
  const groupAddresses = input.scope === "complete" || input.scope === "partial-group-addresses" || input.scope === "partial-both";
  const parameters = input.scope === "complete" || input.scope === "partial-parameters" || input.scope === "partial-both";
  if (groupAddresses && input.plan.groupLinks > 0) checks.push("groupAddresses", "status");
  if (parameters) checks.push("parameters");
  if (parameters && planWarnings(input.warnings).some((warning) => warning.rule === "base-image-low-octet-first")) {
    checks.push("multiOctet");
  }
  return checks;
}
