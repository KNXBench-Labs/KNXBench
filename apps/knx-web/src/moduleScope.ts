/** Response-local module scope identity (ADR-0063); never a durable ID or write target. */
import type { ModuleScope } from "./api";

/**
 * The scope's `nodeChain` when it has the published shape: nonempty safe
 * integers, outermost first, ending in `moduleNode`. Anything else (an older
 * server sends none) is `null`, so callers fall back to the legacy fields
 * instead of trusting or refusing an unexpected value.
 */
export function moduleScopeChain(scope: ModuleScope): readonly number[] | null {
  const chain: unknown = scope.nodeChain;
  if (!Array.isArray(chain) || chain.length === 0) return null;
  if (!chain.every((node) => Number.isSafeInteger(node))) return null;
  return chain[chain.length - 1] === scope.moduleNode ? chain as number[] : null;
}

/**
 * Equal strings mean the same evaluation scope within one response. With a
 * chain, nested nameless scopes sharing every legacy field stay apart; the
 * tagged legacy fallback can never equal a chained identity.
 */
export function moduleScopeIdentity(scope: ModuleScope | null): string {
  if (scope === null) return JSON.stringify(null);
  const chain = moduleScopeChain(scope);
  return JSON.stringify(chain === null
    ? ["legacy", scope.moduleNode, scope.moduleId, scope.moduleDefId]
    : ["chain", chain, scope.moduleId, scope.moduleDefId]);
}
