/** Creates opaque presentation lifetimes on localhost and non-secure LAN HTTP origins. */
// These are channel/project-scope identities, never authentication credentials.
// getRandomValues remains available where secure-context-only randomUUID is not.
export function createFlowScope(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(16));
  return Array.from(bytes, byte => byte.toString(16).padStart(2, "0")).join("");
}
