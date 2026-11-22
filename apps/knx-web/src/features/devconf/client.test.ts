import { describe, expect, it, vi } from "vitest";
import { createDevconfClient, DevconfContractError, type DevconfAction, type DevconfContext } from "./client";

const context: DevconfContext = { serverIncarnation: "srv", projectIncarnation: 1, snapshotRevision: 7 };

function jsonResponse(value: unknown, ok = true, status = ok ? 200 : 400): Response {
  return new Response(JSON.stringify(value), { status, headers: { "Content-Type": "application/json" } });
}

const validProfile = { device: 3, moduleId: "smartsensor", moduleVersion: "1.0.0", moduleSha256: "a".repeat(64), documentBytes: 14, documentSha256: "b".repeat(64) };
const validView = { ...context, profiles: [validProfile], persistence: "native-working-history" };

const setAction: DevconfAction = { kind: "set", draft: { device: 3, moduleId: "smartsensor", moduleVersion: "1.0.0", moduleSha256: "a".repeat(64), documentB64: "eyJub2RlcyI6W119" } };

describe("devconf client", () => {
  it("reads and admits a valid view", async () => {
    const transport = vi.fn().mockResolvedValue(jsonResponse(validView));
    const client = createDevconfClient(transport as unknown as typeof fetch);
    await expect(client.read()).resolves.toEqual(validView);
    expect(transport).toHaveBeenCalledWith("/api/project/device-config-profiles", expect.objectContaining({ method: "GET" }));
  });

  it("rejects a view with duplicate devices", async () => {
    const transport = vi.fn().mockResolvedValue(jsonResponse({ ...validView, profiles: [validProfile, validProfile] }));
    const client = createDevconfClient(transport as unknown as typeof fetch);
    await expect(client.read()).rejects.toBeInstanceOf(DevconfContractError);
  });

  it("rejects an uppercase sha256", async () => {
    const bad = { ...validView, profiles: [{ ...validProfile, moduleSha256: "A".repeat(64) }] };
    const transport = vi.fn().mockResolvedValue(jsonResponse(bad));
    const client = createDevconfClient(transport as unknown as typeof fetch);
    await expect(client.read()).rejects.toBeInstanceOf(DevconfContractError);
  });

  it("previews a set action and echoes the action", async () => {
    const preview = { token: "tok", action: setAction, previous: validProfile, noOp: false, expiresInSeconds: 300 };
    const transport = vi.fn().mockResolvedValue(jsonResponse(preview));
    const client = createDevconfClient(transport as unknown as typeof fetch);
    await expect(client.preview(context, setAction)).resolves.toEqual(preview);
    const call = transport.mock.calls[0][1] as { body: string };
    const sent = JSON.parse(call.body);
    expect(sent.action).toEqual(setAction);
    expect(sent.snapshotRevision).toBe(7);
  });

  it("accepts an equal-length changed document as a real change but refuses an impossible no-op", async () => {
    const sameLength: DevconfAction = { kind: "set", draft: { device: 3, moduleId: "smartsensor", moduleVersion: "1.0.0", moduleSha256: "a".repeat(64), documentB64: encodeOf("x".repeat(14)) } };
    const changed = { token: "tok", action: sameLength, previous: validProfile, noOp: false, expiresInSeconds: 300 };
    const client = createDevconfClient(vi.fn().mockResolvedValue(jsonResponse(changed)) as unknown as typeof fetch);
    await expect(client.preview(context, sameLength)).resolves.toEqual(changed);
    const shorter: DevconfAction = { kind: "set", draft: { device: 3, moduleId: "smartsensor", moduleVersion: "1.0.0", moduleSha256: "a".repeat(64), documentB64: encodeOf("x") } };
    const impossible = { token: "tok", action: shorter, previous: validProfile, noOp: true, expiresInSeconds: 300 };
    const bad = createDevconfClient(vi.fn().mockResolvedValue(jsonResponse(impossible)) as unknown as typeof fetch);
    await expect(bad.preview(context, shorter)).rejects.toBeInstanceOf(DevconfContractError);
  });

  it("reads one device's stored document and admits it strictly", async () => {
    const text = '{ "a" :1 }';
    const { documentBytes: _unused, ...rest } = validProfile;
    void _unused;
    const device = { ...context, device: 3, persistence: "native-working-history", profile: { ...rest, documentBytes: text.length, documentB64: encodeOf(text), documentError: null } };
    const transport = vi.fn().mockResolvedValue(jsonResponse(device));
    const client = createDevconfClient(transport as unknown as typeof fetch);
    await expect(client.readDevice(3)).resolves.toEqual(device);
    expect(transport).toHaveBeenCalledWith("/api/project/device-config-profiles/devices/3", expect.objectContaining({ method: "GET" }));
    for (const broken of [
      { ...device, device: 4 },
      { ...device, profile: { ...device.profile, documentBytes: text.length + 1 } },
      { ...device, profile: { ...device.profile, documentB64: "***" } },
      { ...device, profile: { ...device.profile, extra: 1 } },
      { ...device, profile: { ...device.profile, documentSha256: "B".repeat(64) } },
    ]) {
      const bad = createDevconfClient(vi.fn().mockResolvedValue(jsonResponse(broken)) as unknown as typeof fetch);
      await expect(bad.readDevice(3)).rejects.toBeInstanceOf(DevconfContractError);
    }
    const none = createDevconfClient(vi.fn().mockResolvedValue(jsonResponse({ ...device, profile: null })) as unknown as typeof fetch);
    await expect(none.readDevice(3)).resolves.toMatchObject({ profile: null });
  });

  it("applies with a confirmed token and admits the outcome", async () => {
    const applied = { changed: true, nativeGeneration: 9, snapshotRevision: 8, persistence: "native-working-history" };
    const transport = vi.fn().mockResolvedValue(jsonResponse(applied));
    const client = createDevconfClient(transport as unknown as typeof fetch);
    await expect(client.apply("tok")).resolves.toEqual(applied);
    const call = transport.mock.calls[0][1] as { body: string };
    expect(JSON.parse(call.body)).toEqual({ token: "tok", confirmed: true });
  });

  it("rejects an applied outcome without native generation on native persistence", async () => {
    const applied = { changed: true, nativeGeneration: null, snapshotRevision: 8, persistence: "native-working-history" };
    const transport = vi.fn().mockResolvedValue(jsonResponse(applied));
    const client = createDevconfClient(transport as unknown as typeof fetch);
    await expect(client.apply("tok")).rejects.toBeInstanceOf(DevconfContractError);
  });
});

function encodeOf(text: string): string {
  return btoa(text).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}
