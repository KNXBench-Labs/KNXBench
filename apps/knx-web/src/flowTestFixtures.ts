/** U20 test fixtures: AR20 flow-snapshot and telegram-row JSON as the server sends it. */

export function snapshotJson(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    serverIncarnation: "inc-1",
    sessionId: 7,
    generation: "1",
    status: "current",
    groupAddressStyle: "ThreeLevel",
    devices: [{ deviceId: 1, installationId: 1, name: "Switch", individualAddressRaw: 0x1101 }],
    groups: [
      {
        gaRaw: 0x0801, gaId: 10, installationId: 1, name: "Light", dpt: "1.001",
        members: [{
          deviceId: 1, comObjectId: 100, direction: "Send", active: true,
          flags: { communication: true, read: null, write: false, transmit: true, update: null, readOnInit: null },
        }],
      },
    ],
    diagnostics: {
      duplicateIndividualAddresses: [], ambiguousGroupAddresses: [], ambiguousDevices: [],
      danglingLinks: [], unknownDevices: [], objectsWithoutFlags: [],
    },
    truncated: { devices: 0, groups: 0, members: 0, diagnostics: 0 },
    ...overrides,
  };
}
