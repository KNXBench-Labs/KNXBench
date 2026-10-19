/** Presentation-only caption safety; positional values are resolved exclusively by the server. */
import type { TextFill } from "./bindings/TextFill";
import type { Translate } from "./i18n";

export interface ParameterCaptionSource {
  label: string | null;
  labelFill?: TextFill | null;
}

/**
 * Conservative display policy: even resolved values may contain literal delimiters.
 * Keep the result and provenance in the tooltip; never parse or substitute here.
 */
export function parameterCaption(source: ParameterCaptionSource, fallback: string, t: Translate) {
  const { label, labelFill } = source;
  const unreadable = !label?.trim() || label.trim() === "#" || label.includes("{{") || label.includes("}}");
  const explanation = labelFill
    ? `${t(`comTable.textFill.${labelFill.source}`)}\n${t("comTable.textFill.template", { value: labelFill.template })}`
    : "";
  return {
    label: unreadable ? fallback : label!,
    title: [unreadable ? t("parameters.caption.unavailable", { value: label ?? "" }) : "", explanation].filter(Boolean).join("\n") || undefined,
  };
}
