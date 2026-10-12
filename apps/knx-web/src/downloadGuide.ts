/** The download tab's five steps, derived only from checks actually made. */

export type GuideStepId = "device" | "gateway" | "scope" | "plan" | "confirm";
export type GuideStepState = "done" | "open" | "blocked";
/** A concrete missing prerequisite, named in the step that needs it. */
export type GuideMissing = "device" | "gateway" | "plan" | "acknowledgement";

export interface GuideInput {
  /** A device with an individual address is selected (only those are listed). */
  deviceSelected: boolean;
  gateway: string;
  /** The plan currently shown; dropped on project, device, gateway or scope change. */
  plan: { untestedAcknowledgement: string | null } | null;
  acknowledgement: string;
}

export interface GuideStep { id: GuideStepId; state: GuideStepState; missing: GuideMissing[] }

export function downloadGuide(input: GuideInput): GuideStep[] {
  const gateway = input.gateway.trim() !== "";
  const acknowledged = input.plan === null || input.plan.untestedAcknowledgement === null
    || input.acknowledgement === input.plan.untestedAcknowledgement;
  const confirmMissing: GuideMissing[] = [];
  if (!gateway) confirmMissing.push("gateway");
  if (!acknowledged) confirmMissing.push("acknowledgement");
  return [
    { id: "device", state: input.deviceSelected ? "done" : "open", missing: input.deviceSelected ? [] : ["device"] },
    { id: "gateway", state: gateway ? "done" : "open", missing: gateway ? [] : ["gateway"] },
    // A scope is always chosen; "complete" is the default.
    { id: "scope", state: "done", missing: [] },
    input.plan !== null ? { id: "plan", state: "done", missing: [] }
      : input.deviceSelected ? { id: "plan", state: "open", missing: ["plan"] }
        : { id: "plan", state: "blocked", missing: ["device"] },
    // Consent, the confirmation phrase, the plan check and the one-tunnel
    // rule happen on Start; this step is never "done" before that.
    { id: "confirm", state: input.plan === null ? "blocked" : "open", missing: input.plan === null ? ["plan"] : confirmMissing },
  ];
}

/** The step to look at next: the first one not done. */
export function currentGuideStep(steps: readonly GuideStep[]): GuideStepId | null {
  return steps.find((step) => step.state !== "done")?.id ?? null;
}
