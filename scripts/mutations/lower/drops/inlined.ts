// Mutations of src/lower/drops/inlined.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "drop-inlined-where-not-each",
    breaks: "a `Drop` with a drop that isn't a call, or of another module, is gone where that one still calls it",
    file: "src/lower/drops/inlined.rs",
    find: "        if !each || inlined == 0 || inlined != mentions {\n",
    replace: "        if inlined == 0 {\n",
    tests: ["test/lowering.test.ts", "-t", "a Drop dropped once"],
  },
  {
    name: "drop-in-closure-kept",
    breaks: "a drop in a closure, `unstable_wrapCallback`'s, isn't its body, and the function stays",
    file: "src/lower/drops/inlined.rs",
    find: "                    js::each_block_mut(&mut made, &mut |stmts| {\n",
    replace: "                    js::statement_lists(&mut made, &mut |stmts| {\n",
    tests: ["test/lowering.test.ts", "-t", "a Drop dropped once"],
  },
  {
    name: "one-field-guard-kept",
    breaks: "a one-field tuple guard stays `const _restore = [level]`, read `_restore[0]`",
    file: "src/lower/drops/inlined.rs",
    find: "                    one_field_of(&mut made, &guard);\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "a Drop dropped once"],
  },
  {
    name: "written-guard-kept",
    breaks: "a guard whose field is written stays `const more = { hasMoreWork: true }`",
    file: "src/lower/drops/inlined.rs",
    find: "    if reads != js::mentions_in(body, guard) {\n        return;\n    }\n    let mut names",
    replace: "    if reads != js::mentions_in(body, guard) || !written.is_empty() {\n        return;\n    }\n    let mut names",
    tests: ["test/lowering.test.ts", "-t", "a Drop dropped once"],
  },
  {
    name: "drop-inlined-twice-dropped",
    breaks: "a `Drop` dropped in two places is its function at each, `loudDrop_drop(_b)`, not its body",
    file: "src/lower/drops/inlined.rs",
    find: "        if !each || inlined == 0 || inlined != mentions {\n",
    replace: "        if !each || inlined == 0 || inlined != mentions || mentions > 1 {\n",
    tests: ["test/lowering.test.ts", "-t", "a Drop dropped once"],
  },
  {
    name: "guard-kept-whole",
    breaks: "a guard made only to be dropped stays an object, `const _restore = { previous: depth }`",
    file: "src/lower/drops/inlined.rs",
    find: "                    fields_of(&mut made, &function.params, &guard);\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "a Drop dropped once"],
  },
  {
    name: "guard-read-whole-taken-apart",
    breaks: "a guard read whole, given to a function, is taken apart into its fields, and the function reads a variable that's gone",
    file: "src/lower/drops/inlined.rs",
    find: "    if written || reads != js::mentions_in(body, guard) {\n",
    replace: "    if written {\n",
    tests: ["test/lowering.test.ts", "-t", "a Drop dropped once"],
  },
];
