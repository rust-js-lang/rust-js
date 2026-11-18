// Mutations of builtins/policy.ts, what TypeScript's library doesn't say of
// a builtin that its Rust binding needs (#1904).
import type { Mutation } from "../../mutations";

const file = "builtins/policy.ts";

export const mutations: Mutation[] = [
  {
    name: "policy-hour-cycle-unnamed",
    breaks: "`Intl.LocaleHourCycleKey` is written under another name than the established `HourCycle`, which the date formatter's options and callers name",
    file,
    find: '    rust: "HourCycle",',
    replace: '    rust: "LocaleHourCycleKey",',
    tests: ["test/builtins-coverage.test.ts", "-t", "Intl is what its template writes"],
  },
  {
    name: "policy-locale-throws-forgotten",
    breaks: "`new Intl.Locale` isn't said to throw, so its `RangeError` of a tag that isn't one isn't a `Result`",
    file,
    find: '  "new Intl.Locale": {\n    error: "RangeError",',
    replace: '  "new Intl.Locale.unused": {\n    error: "RangeError",',
    tests: ["test/intl-locale.test.ts", "-t", "a locale of a tag or of another locale"],
  },
];
