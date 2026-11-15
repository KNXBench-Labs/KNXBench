/** Shape rules for the explicit-TCP `connection` facts of comparison and diagnostics responses. */

export interface TcpConnectionFacts {
  requestedTransport: "tcp";
  actualTransport: "tcp" | null;
  assignedAddress: string | null;
  skippedFramesBeforeDisconnect: number | null;
}

const individualAddress = (v: unknown) => typeof v === "string" && /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(v) &&
  v.split(".").every((part, index) => Number(part) <= (index === 2 ? 255 : 15));

/**
 * Missing is not zero and selected is not actual: no tunnel means every fact is null together,
 * an opened one carries all of them. `requireActual` refuses "no tunnel" where success is claimed.
 */
export function validTcpConnection(c: unknown, requireActual: boolean): c is TcpConnectionFacts {
  if (!c || typeof c !== "object" || Array.isArray(c)) return false;
  const f = c as TcpConnectionFacts;
  return f.requestedTransport === "tcp" &&
    (f.actualTransport === "tcp" || f.actualTransport === null) &&
    (f.assignedAddress === null || individualAddress(f.assignedAddress)) &&
    (f.skippedFramesBeforeDisconnect === null || (Number.isSafeInteger(f.skippedFramesBeforeDisconnect) && f.skippedFramesBeforeDisconnect >= 0)) &&
    (f.actualTransport === null ? f.assignedAddress === null && f.skippedFramesBeforeDisconnect === null
      : f.assignedAddress !== null && f.skippedFramesBeforeDisconnect !== null) &&
    !(requireActual && f.actualTransport !== "tcp");
}
