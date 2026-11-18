// Mutations of src/lower/mir/iter/copies.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "cloned-strings-mapped",
    breaks: "`cloned()` of strings maps each to itself",
    file: "src/lower/mir/iter/copies.rs",
    find: "                .is_some_and(|item| !self.needs_clone(item.peel_refs()))\n",
    replace:
      "                .is_some_and(|item| self.is_copy(item.peel_refs()))\n",
    tests: ["test/corpus.test.ts", "-t", "^(copied_structs_apart).rs$"],
  },
  {
    name: "cloned-mapped-anyway",
    breaks: "a `cloned()` of what needs no clone is mapped as one that does",
    file: "src/lower/mir/iter/copies.rs",
    find: "            || crate::lower::recognition::cloned_source(self.tcx, ty).is_some())\n",
    replace: "            )\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(consumed_adapter_drops|cloned_items_from_both_ends).rs$",
    ],
    snapshots: true,
  },
  {
    name: "copied-items-shared",
    breaks:
      "`copied()` is the iterator it copies, its `Copy` structs the same objects, so changing a copy changes the slice's",
    file: "src/lower/mir/iter/copies.rs",
    find: "            || crate::lower::recognition::cloned_source(self.tcx, ty).is_some())\n            && item()",
    replace:
      "            || crate::lower::recognition::cloned_source(self.tcx, ty).is_some())\n            && (crate::lower::recognition::copied_source(self.tcx, ty).is_some() || item())",
    tests: ["test/corpus.test.ts", "-t", "^copied_structs_apart.rs$"],
  },
  {
    name: "copied-pointee-peeled",
    breaks:
      "`copied()` of an iterator of references to references copies what the inner one points at, `{ ...item }`, a set of `&Fiber` keys of copies",
    file: "src/lower/mir/iter/copies.rs",
    find: "        let pointee = match *item.kind() {\n            ty::Ref(_, pointee, _) => pointee,\n            _ => item,\n        };\n",
    replace: "        let pointee = item.peel_refs();\n",
    tests: ["test/corpus.test.ts", "-t", "^identity_keys.rs$"],
  },
  {
    name: "copied-items-mapped-backless",
    breaks: "a `cloned()` of a chain the crate steps from its back, which knows no length, is JS's `map` of its clones, which has no back, and the back step throws (#850)",
    file: "src/lower/mir/iter/copies.rs",
    find: "        let back = self.knows_its_back(output);\n",
    replace: "        let back = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^cloned_of_a_chain_stepped_from_its_back.rs$"],
  },
  {
    name: "copied-source-unknowing",
    breaks: "a `cloned()` or a `copied()` made knowing its back is `$mapEnds` of its source as it is, a `step_by(2)`'s forward-only `values().filter(..)`, whose `rfold` throws `it.back is not a function` (#851)",
    file: "src/lower/mir/iter/copies.rs",
    find: "            Some(source) if back => self.as_double_ended(it, source.peel_refs(), span)?,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^copies_of_forward_forms_stepped_from_their_back.rs$"],
  },
  {
    name: "copied-made-unparted",
    breaks: "generic code's `copied()` the crate clones isn't made by a function of its source, so its clone is `$clone` of a map that carries none, and throws (#1181)",
    file: "src/lower/mir/iter/copies.rs",
    find: "        if !self.iterator_cloned_apart(output) {\n            return self.copied_items(it, item, output, span);\n",
    replace: "        if true {\n            return self.copied_items(it, item, output, span);\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_copied_cloned.rs$"],
  },
  {
    name: "copied-calls-its-clone",
    breaks: "`copied()` of a `Copy` type with a hand-written `Clone` calls that `clone` for each item, which std's `Copied` never does",
    file: "src/lower/mir/iter/copies.rs",
    find: "            Some(_) => self.copy_if_needed(Expr::var(&x), pointee),\n",
    replace: "            Some(_) => self.clone_value(Expr::var(&x), pointee, span, &mut body)?,\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_of_a_copy_with_its_own_clone.rs$"],
  },
];
