/** Visible render-window notice; unrendered matches are never presented as absent. */
import { useTranslate } from "./i18n";
export const GROUP_ADDRESS_WINDOW = 500;

export default function GroupAddressWindowNotice(props: { shown: number; total: number; onLimit: (limit: number) => void }) {
  const t = useTranslate();
  if (props.shown >= props.total) return null;
  return <p className="ga-window" role="status" data-ga-window>
    {t("gaWorkspace.window", { shown: props.shown, total: props.total })}{" "}
    <button type="button" data-ga-more onClick={() => props.onLimit(props.shown + GROUP_ADDRESS_WINDOW)}>{t("gaWorkspace.showMore", { count: Math.min(GROUP_ADDRESS_WINDOW, props.total - props.shown) })}</button>{" "}
    <button type="button" data-ga-all onClick={() => props.onLimit(Number.POSITIVE_INFINITY)}>{t("gaWorkspace.showAll", { total: props.total })}</button>
  </p>;
}
