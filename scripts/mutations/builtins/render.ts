// Mutations of builtins/render.ts, the js crate's binding forms the
// builtins' generators write by (#1904).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "classed-not-an-object",
    breaks: "an `Intl` class's instances are no object's binding, so `js::unknown(locale)` is refused, as the vercel-ui SDK's `tag as Intl.Locale & { weekInfo?: .. }` needs (ADR 1172, amended)",
    file: "builtins/render.ts",
    find: '  for (const marker of objectMarkers(name, "super::")) line(marker);\n',
    replace: "",
    tests: ["test/builtins-coverage.test.ts", "-t", "Intl is what its template writes"],
  },
];
