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
  {
    name: "mir-open-drop-other-variant",
    breaks: "an enum some of which has moved drops what's left of a variant when it's another",
    file: "src/lower/mir/drops.rs",
    find: "            let test = self.variant_test(subject.clone(), ty, adt, variant, span)?;\n",
    replace: "            let test = Expr::unary(js::UnaryOp::Not, self.variant_test(subject.clone(), ty, adt, variant, span)?);\n",
    tests: ["test/mir.test.ts","-t","drop_partial|struct_update_drops|temporaries_taken_apart"],
  },
  {
    name: "mir-drop-behind-pointer-skipped",
    breaks: "`self.inner = value` of a generic `T` drops the old value unrun",
    file: "src/lower/mir/drops.rs",
    find: "                    behind.insert(block);\n",
    replace: "",
    tests: ["test/mir.test.ts","-t","drop_generic"],
  },
  {
    name: "mir-cleanup-write-dropped",
    breaks: "a cleanup's write of the replacing value is left out",
    file: "src/lower/mir/drops.rs",
    find: "                            .and_then(|()| self.mir_assign(state, place, rvalue, span, &mut out));\n",
    replace: "                            .and_then(|()| Ok(()));\n",
    tests: ["test/mir.test.ts","-t","drop_replace_panics"],
  },
];
