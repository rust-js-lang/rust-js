// Mutations of src/lower/mir/drops.rs (ADR 0364).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-conditional-drop-static",
    breaks: "a value that may have moved is dropped as if it's always there",
    file: "src/lower/mir/drops.rs",
    find: "            (true, true, false) => Style::Conditional,\n",
    replace: "            (true, true, false) => Style::Static,\n",
    tests: ["test/mir.test.ts", "-t", "drop|refcell_"],
  },
  {
    name: "mir-unwind-drops-nothing",
    breaks: "a panic drops nothing as it unwinds",
    file: "src/lower/mir/drops.rs",
    find: "                    self.mir_drop(state, block, place, true, span, &mut out)?;\n",
    replace: "",
    tests: ["test/mir.test.ts", "-t", "drop|refcell_"],
  },
  {
    name: "mir-unwind-never-caught",
    breaks: "what may panic is never in a `try`, so nothing is dropped as a panic unwinds",
    file: "src/lower/mir/drops.rs",
    find: "        if !body.iter().any(may_throw) {\n",
    replace: "        if true {\n",
    tests: ["test/mir.test.ts", "-t", "drop|refcell_"],
  },
  {
    name: "mir-flags-never-set",
    breaks: "a drop flag keeps its first value, so what moved is dropped or what's there isn't",
    file: "src/lower/mir/drops.rs",
    find: "            if flagged[path] {\n",
    replace: "            if false {\n",
    tests: ["test/mir.test.ts", "-t", "drop|refcell_"],
  },
];
