// Mutations of crates/rust-js-lower/src/iterators/hint_asks/unknown_extends.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "generic-vec-extend-unrefused",
    breaks: "a caller that gives generic code's `extend` a `Vec` and an iterator whose hint is the crate's own isn't refused, so the dictionary asks no hint where std asks it as the `Vec` fills (a wrong answer, #849)",
    file: "crates/rust-js-lower/src/iterators/hint_asks/unknown_extends.rs",
    find: "                    if recognition.hint_reaches_own(iter_ty) && refused.insert(span) {\n",
    replace: "                    if false && refused.insert(span) {\n",
    tests: ["test/diagnostics.test.ts", "-t", "generic code's extend of a Vec its caller gives"],
  },
  {
    name: "generic-extend-unrelayed",
    breaks: "what generic code extends of a caller's type parameters isn't relayed, so the caller that knows them isn't refused (a wrong answer, #849)",
    file: "crates/rust-js-lower/src/iterators/hint_asks/unknown_extends.rs",
    find: "                    changed |= held.insert((collection, iterable));\n",
    replace: "",
    tests: ["test/diagnostics.test.ts", "-t", "generic code's extend relayed"],
  },
  {
    name: "closure-extend-unmerged",
    breaks: "a closure's `extend` isn't the function's it's written in, so a caller of that function isn't refused (a wrong answer, #849)",
    file: "crates/rust-js-lower/src/iterators/hint_asks/unknown_extends.rs",
    find: "                    changed |= theirs.insert(each);\n",
    replace: "",
    tests: ["test/diagnostics.test.ts", "-t", "generic code's extend in a closure"],
  },
  {
    name: "kept-generic-extend-refused",
    breaks: "in a crate that keeps capacities, generic code's `extend` by an iterator its caller gives is refused at its caller, where it grows as std's does (#1180)",
    file: "crates/rust-js-lower/src/iterators/hint_asks/unknown_extends.rs",
    find: "    if keeps_capacity {\n        return;\n    }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^extend_asks_hint_capacity_grown.rs$"],
  },
  {
    name: "dictionary-extend-unrelayed",
    breaks: "generic code's `extend` of a collection its caller gives, in a crate that keeps no capacities, isn't relayed, so the caller that gives a `Vec` and the crate's own hint isn't refused (a wrong answer, #849)",
    file: "crates/rust-js-lower/src/iterators/hint_asks/unknown_extends.rs",
    find: "            let given = extend && recognition.is_unknown(collection);\n",
    replace: "            let given = false && extend && recognition.is_unknown(collection);\n",
    tests: ["test/diagnostics.test.ts", "-t", "generic code's extend"],
  },
  {
    name: "extend-of-a-dyn-keeps-no-capacity",
    breaks: "a crate that extends a `Vec` by a Rust-made `dyn Iterator` keeps no capacities, so its `extend` never asks the hint std asks whenever it's full, the crate's own `size_hint` behind it never runs and an endless one never overflows (#1377)",
    file: "crates/rust-js-lower/src/iterators/hint_asks/unknown_extends.rs",
    find: "                    self.hint_reaches_own_where(iter_ty, !binding)\n",
    replace: "                    self.hint_reaches_own(iter_ty)\n",
    tests: ["test/corpus.test.ts", "-t", "^extend_by_a_dyn_iterator_asks_its_hint.rs$"],
  },
  {
    name: "binding-made-in-mir-never",
    breaks: "a crate that extends by a binding's iterator, JS's own, is taken for one extended by a `dyn` Rust made, so it keeps capacities and each push reads `$grew` where it read `push` (#1377)",
    file: "crates/rust-js-lower/src/iterators/hint_asks/unknown_extends.rs",
    find: "                    if crate::bindings::is_binding(self.tcx, callee) {\n                        return true;\n",
    replace: "                    if crate::bindings::is_binding(self.tcx, callee) {\n                        return false;\n",
    tests: ["test/js-collections.test.ts", "-t", "forEach and iterators"],
  },
];
