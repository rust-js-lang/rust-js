// The test map's item layer (scripts/testMapItems.ts, #2288): which of the
// map's functions reach each item outside any function, each `impl`'s
// header, each function's own name and the unit tests' modules.
import type { Mutation } from "../mutations";

const file = "scripts/testMapItems.ts";
const tests = ["test/testMap-items.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "item-named-by-item-unreached",
    breaks: "an item named only by another item outside any function, `passes: JUMPS` in a const `STAGES` a function iterates, is reached by no function, and a change to it selects nothing (#2288)",
    file,
    find: "  for (let changed = true; changed; ) {\n",
    replace: "  for (let changed = false; changed; ) {\n",
    tests,
  },
  {
    name: "impl-header-trait-methods-unasked",
    breaks: "an `impl`'s header reaches only what names its trait, not what calls its methods, `k.m()` of `Tr#m().`, so a change to its bounds selects too little (#2288)",
    file,
    find: "        for (const f of reaching.get(o.symbol) ?? referencing.get(o.symbol) ?? []) refs.add(f);\n",
    replace: "        for (const f of referencing.get(o.symbol) ?? []) refs.add(f);\n",
    tests,
  },
  {
    name: "new-file-macro-trusted",
    breaks: "a new file that defines a macro selects by its names, though the macro's expansions may be anywhere (#2288)",
    file,
    find: '  if (/\\bmacro_rules!|#\\[proc_macro/.test(code)) return "it defines a macro, whose expansions may be anywhere";\n',
    replace: "",
    tests,
  },
  {
    name: "unit-test-modules-unrecorded",
    breaks: "a `#[cfg(test)]` module isn't recorded, so a name a `use` brings in that its tests name gives everything (#2288)",
    file,
    find: "          tests.push({ file: document.path, span: [at + 1, line + 1] });\n",
    replace: "",
    tests,
  },
];
