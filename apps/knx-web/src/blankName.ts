/** Blank-name test shared by the missing-name wizard and every device-name display. */

/** Blank exactly as `knx_core::names::is_blank_name` (Rust `trim`: the Unicode White_Space set). */
export function isBlankName(text: string): boolean {
  return /^\p{White_Space}*$/u.test(text);
}
