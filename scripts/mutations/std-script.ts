// Mutations of scripts/std.ts, std built and its sysroot linked (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "std-sysroot-linked-in-place",
    breaks: "std's sysroot is linked where it's read, not made whole beside it and moved in, so a run that finds it half linked reads it, or links into it as another does (`EEXIST`, #2459)",
    file: "scripts/std.ts",
    find: '  const making = mkdtempSync(join(dir, ".sysroot-"));\n',
    replace: "  const making = sysroot;\n",
    tests: ["test/std-sites.test.ts", "-t", "many runs at once"],
  },
];
