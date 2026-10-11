/** Invalid persisted opt-out values cannot authorize an external network request. */
import { expect, it } from "vitest";
import { releaseChecksEnabled } from "./releaseUpdatePreference";
it.each([false, "false", "true", 0, 1, null, {}, []])("fails closed for %j", value => expect(releaseChecksEnabled(value)).toBe(false));
it.each([undefined, true])("enables only an absent preference or boolean true", value => expect(releaseChecksEnabled(value)).toBe(true));
