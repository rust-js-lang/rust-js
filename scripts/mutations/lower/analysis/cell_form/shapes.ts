// Mutations of crates/rust-js-lower/src/analysis/cell_form/shapes.rs (#2490:
// what `CellForm`'s proof reads of a body's THIR, ADR 0863's case N).
import type { Mutation } from "../../../../mutations";

const file = "crates/rust-js-lower/src/analysis/cell_form/shapes.rs";

export const mutations: Mutation[] = [
  {
    name: "unmet-of-a-float",
    breaks:
      "a `contains` of a float guards the push as a string's does, though `NaN` isn't equal to itself: the push's borrow of the same cell isn't checked, and runs where Rust's is busy (ADR 0863, amended)",
    file,
    find: "        let reflexive = recognition.is_string_like(element) || element.is_integral() || element.is_bool();\n",
    replace: "        let reflexive = true;\n",
    tests: ["test/corpus.test.ts", "-t", "^contains_guarded_float.rs$"],
  },
  {
    name: "unmet-of-any-value",
    breaks:
      "a `contains` of any value guards the push, not only of the loop's item, which is there where the two are one cell (ADR 0863, amended)",
    file,
    find: "        if !items.contains(&item) || !reflexive {\n",
    replace: "        if !reflexive {\n",
    tests: ["test/corpus.test.ts", "-t", "^contains_guarded_other.rs$"],
  },
  {
    name: "unmet-never",
    breaks:
      "React's scheduleGesture's push only where a type isn't there yet is counted, `$borrowMut`, as any borrow that may meet the loop's is",
    file,
    find: "                && self.unmet_in(recognition, cond, &items) == Some(var)\n",
    replace: "                && self.unmet_in(recognition, cond, &items) == Some(var)\n                && false\n",
    tests: ["test/corpus.test.ts", "-t", "^contains_guarded_borrow.rs$"],
    snapshots: true,
  },
  {
    name: "unmet-of-a-handle-set-again",
    breaks:
      "a handle the body sets again is one cell wherever it's read, so its borrow after the test may be of another, `own`'s",
    file,
    find: "        (!changed).then_some(var)\n",
    replace: "        Some(var)\n",
    tests: ["test/corpus.test.ts", "-t", "^contains_guarded_set_again.rs$"],
  },
];
