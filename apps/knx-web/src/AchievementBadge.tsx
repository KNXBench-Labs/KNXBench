/** A round achievement badge: the tier's colour around a workbench glyph. */
import type { AchievementTier } from "./achievementCatalog";
import WorkbenchIcon, { type WorkbenchIconName } from "./WorkbenchIcon";

export default function AchievementBadge(props: { tier: AchievementTier; glyph: WorkbenchIconName; locked?: boolean }) {
  const className = `achievement-badge achievement-badge--${props.tier}${props.locked ? " achievement-badge--locked" : ""}`;
  return (
    <span className={className} aria-hidden="true">
      <WorkbenchIcon name={props.glyph} />
    </span>
  );
}
