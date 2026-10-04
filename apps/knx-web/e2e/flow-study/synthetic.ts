/** U19 study: a deterministic synthetic installation and telegram stream; never real bus data. */
import type { StudyEvent } from "./model";

export interface SyntheticGroup { ga: string; sender: string; targets: string[] }
export interface SyntheticInstallation { devices: string[]; groups: SyntheticGroup[]; directedEdges: number }

function random(seed: number): () => number {
  let state = seed >>> 0 || 1;
  return () => {
    state ^= state << 13; state >>>= 0;
    state ^= state >>> 17;
    state ^= state << 5; state >>>= 0;
    return state / 0x1_0000_0000;
  };
}

/** `9.x.y` addresses mark every device as synthetic. One group in ten has no
 * configured member and shows as an unresolved group-address node. */
export function syntheticInstallation(seed: number, deviceCount: number, edgeTarget: number): SyntheticInstallation {
  const next = random(seed);
  const devices = Array.from({ length: deviceCount }, (_, i) => `9.${Math.floor(i / 250) + 1}.${(i % 250) + 1}`);
  const groups: SyntheticGroup[] = [];
  let directedEdges = 0;
  const pairs = new Set<string>();
  let attempts = 0;
  while (directedEdges < edgeTarget && attempts < edgeTarget * 20) {
    attempts += 1;
    const n = groups.length;
    const ga = `${Math.floor(n / 2048) % 32}/${Math.floor(n / 256) % 8}/${n % 256}`;
    const sender = devices[Math.floor(next() * deviceCount)];
    const targets: string[] = [];
    if (next() >= 0.1) {
      const count = 1 + Math.floor(next() * 3);
      for (let i = 0; i < count; i += 1) {
        const target = devices[Math.floor(next() * deviceCount)];
        if (target !== sender && !targets.includes(target)) targets.push(target);
      }
    }
    const ends = targets.length > 0 ? targets : [`GA:${ga}`];
    const fresh = ends.filter((end) => !pairs.has(`${sender}→${end}`));
    if (fresh.length === 0) continue;
    fresh.forEach((end) => pairs.add(`${sender}→${end}`));
    directedEdges += fresh.length;
    groups.push({ ga, sender, targets });
  }
  return { devices, groups, directedEdges };
}

/** Events at a fixed rate from `startMs`; group choice is skewed so a few
 * groups are hot, like real installations with chatty sensors. */
export function eventStream(installation: SyntheticInstallation, seed: number, ratePerSecond: number, startMs: number) {
  const next = random(seed ^ 0x5eed);
  let emitted = 0;
  return {
    get emitted() { return emitted; },
    /** Every event whose scheduled time is at or before `nowMs`. */
    due(nowMs: number): StudyEvent[] {
      // Event i is scheduled at startMs + i * period and due from that instant.
      const owed = nowMs < startMs ? 0 : Math.floor(((nowMs - startMs) * ratePerSecond) / 1000) + 1;
      const batch: StudyEvent[] = [];
      while (emitted < owed) {
        const scheduled = startMs + (emitted * 1000) / ratePerSecond;
        const group = installation.groups[Math.floor(next() ** 2.2 * installation.groups.length)];
        const roll = next();
        emitted += 1;
        if (roll < 0.8 || group.targets.length === 0) {
          batch.push({ seq: emitted, observedAtMs: scheduled, source: group.sender, ga: group.ga, service: "GroupValueWrite",
            value: next() < 0.5 ? "On" : `${(next() * 30).toFixed(1)} °C`, targets: group.targets });
        } else if (roll < 0.9) {
          batch.push({ seq: emitted, observedAtMs: scheduled, source: group.sender, ga: group.ga, service: "GroupValueRead", targets: group.targets });
        } else {
          // An independently sourced response: one configured member answers.
          batch.push({ seq: emitted, observedAtMs: scheduled, source: group.targets[0], ga: group.ga, service: "GroupValueResponse",
            value: `${(next() * 100).toFixed(0)} %`, targets: [group.sender] });
        }
      }
      return batch;
    },
  };
}
