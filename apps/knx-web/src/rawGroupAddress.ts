/** Reads the project's group-address display style to parse projected addresses into raw values. */
// Projected addresses arrive formatted per the project's `GroupAddressStyle`; the
// server's link comparison names raw 16-bit values, so marks need the inverse map.
/** Parses a projected group address into its raw 16-bit value, or null when unparsable. */
export function rawGroupAddress(address: string, style: string): number | null {
  const parts = address.split("/");
  if (style === "Free") {
    const raw = Number(address);
    return Number.isInteger(raw) && raw >= 0 && raw <= 65535 ? raw : null;
  }
  if (style === "TwoLevel") {
    if (parts.length !== 2) return null;
    const main = Number(parts[0]), sub = Number(parts[1]);
    return Number.isInteger(main) && Number.isInteger(sub) && main >= 0 && main <= 31 && sub >= 0 && sub <= 2047
      ? (main << 11) | sub : null;
  }
  if (parts.length !== 3) return null;
  const main = Number(parts[0]), middle = Number(parts[1]), sub = Number(parts[2]);
  return Number.isInteger(main) && Number.isInteger(middle) && Number.isInteger(sub)
    && main >= 0 && main <= 31 && middle >= 0 && middle <= 7 && sub >= 0 && sub <= 255
    ? (main << 11) | (middle << 8) | sub : null;
}
