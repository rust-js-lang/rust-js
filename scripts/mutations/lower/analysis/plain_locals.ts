// Mutations of src/lower/analysis/plain_locals.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "plain-locals-boxed",
    breaks: "a thread-local only read and set in its own module keeps its `{ value }`, `const COUNT = { value: 0 }`",
    file: "src/lower/analysis/plain_locals.rs",
    find: "        .filter(|&key| !tcx.visibility(key).is_public())\n",
    replace: "        .filter(|_| false)\n",
    tests: ["test/modules.test.ts", "-t", "only read and set is the module's"],
  },
  {
    name: "other-module-local-unboxed",
    breaks: "a thread-local another module reads is its module's `let`, which the importer can't set",
    file: "src/lower/analysis/plain_locals.rs",
    find: "if !accessed.contains(&id) || tcx.parent_module_from_def_id(key) != module {",
    replace: "if !accessed.contains(&id) {",
    tests: ["test/modules.test.ts", "-t", "only read and set is the module's"],
  },
  {
    name: "set-local-const",
    breaks: "a thread-local that's `set` is a `const`, which JS can't assign",
    file: "src/lower/analysis/plain_locals.rs",
    find: "                if sets {\n",
    replace: "                if false && sets {\n",
    tests: ["test/modules.test.ts", "-t", "only read and set is the module's"],
  },
  {
    name: "read-at-once-past-calls",
    breaks: "a `Cell` read after a call is its value from the `let`, so `title.get()` after `error.title.set(..)` reads the old title",
    file: "src/lower/analysis/plain_locals.rs",
    find: "            if runs && !self.waiting.is_empty() {",
    replace: "            if false && !self.waiting.is_empty() {",
    tests: ["test/lowering.test.ts", "-t", "a Cell a let takes apart"],
  },
  {
    name: "read-at-once-past-blocks",
    breaks: "a `Cell` read after a block is its value from the `let`, so what a drop in the block set is missed",
    file: "src/lower/analysis/plain_locals.rs",
    find: "                ExprKind::Block { .. } => true,\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "a Cell a let takes apart"],
  },
  {
    name: "read-at-once-in-loops",
    breaks: "a `Cell` read in a loop is its value from the `let`, so the second turn reads the title the first set",
    file: "src/lower/analysis/plain_locals.rs",
    find: "                self.early &= self.loops == 0;\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "a Cell a let takes apart"],
  },
  {
    name: "read-at-once-other-uses",
    breaks: "a `&Cell` used otherwise too is its value, so `let other = title; other.set(..)` sets a string's property",
    file: "src/lower/analysis/plain_locals.rs",
    find: "                if uses.get(&var) != Some(&reads.len()) {",
    replace: "                if false {",
    tests: ["test/lowering.test.ts", "-t", "a Cell a let takes apart"],
  },
  {
    name: "read-at-once-builders-run",
    breaks: "building an element counts as running code, so a `Cell` JSX reads is never its value: ErrorMessage reads `error.title` where the original destructures it",
    file: "src/lower/analysis/plain_locals.rs",
    find: "                    !fn_def(self.thir[fun].ty).is_some_and(|(def, _)| is_element_builder(self.tcx, def))",
    replace: "                    true",
    tests: ["test/jsx.test.ts", "-t", "JSX reading a Cell"],
  },
];
