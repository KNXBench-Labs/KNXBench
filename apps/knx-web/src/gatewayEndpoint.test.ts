/** Tests lossless gateway-field separation and the current IPv4-only tunnel boundary. */
import { describe, expect, it } from "vitest";
import { splitGatewayEndpoint, validateGatewayFields } from "./gatewayEndpoint";

describe("gateway endpoint fields", () => {
  it("separates a stored IPv4 endpoint and defaults a missing port", () => {
    expect(splitGatewayEndpoint(" 192.0.2.10:4921 ")).toEqual({ host: "192.0.2.10", port: "4921" });
    expect(splitGatewayEndpoint("192.0.2.10")).toEqual({ host: "192.0.2.10", port: "3671" });
    expect(splitGatewayEndpoint("192.0.2.10:")).toEqual({ host: "192.0.2.10", port: "3671" });
    expect(splitGatewayEndpoint("")).toEqual({ host: "", port: "3671" });
  });

  it("does not discard unsupported hostname and IPv6 preferences", () => {
    expect(splitGatewayEndpoint("gateway.local:4921")).toEqual({ host: "gateway.local", port: "4921" });
    expect(splitGatewayEndpoint("[2001:db8::1]:4921")).toEqual({ host: "2001:db8::1", port: "4921" });
    expect(splitGatewayEndpoint("2001:db8::1")).toEqual({ host: "2001:db8::1", port: "3671" });
  });

  it("composes only a valid numeric IPv4 host and port for the server", () => {
    expect(validateGatewayFields({ host: " 192.0.2.10 ", port: "" })).toEqual({ ok: true, endpoint: "192.0.2.10:3671" });
    expect(validateGatewayFields({ host: "0.0.0.0", port: "65535" })).toEqual({ ok: true, endpoint: "0.0.0.0:65535" });
    expect(validateGatewayFields({ host: "192.0.2.10", port: "03671" })).toEqual({ ok: true, endpoint: "192.0.2.10:3671" });
  });

  it("rejects malformed ports instead of sending a request", () => {
    for (const port of ["0", "65536", "-1", "1.5", "abc", "12e2", " 1 2 "]) {
      expect(validateGatewayFields({ host: "192.0.2.10", port })).toEqual({ ok: false, reason: "invalidPort" });
    }
  });

  it("reports an absent or unsupported host before any connection", () => {
    expect(validateGatewayFields({ host: " ", port: "3671" })).toEqual({ ok: false, reason: "hostRequired" });
    for (const host of ["gateway.local", "2001:db8::1", "[2001:db8::1]", "999.0.0.1", "192.168.01.2", "192.0.2.1:3671"]) {
      expect(validateGatewayFields({ host, port: "3671" })).toEqual({ ok: false, reason: "unsupportedHost" });
    }
  });
});
