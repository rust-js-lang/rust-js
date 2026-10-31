// Mutations of src/runtime.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "pure-helpers-act",
    breaks: "the runtime's helpers that only read, `$jsonIndex(v, \"a\")`, count as doing something, held apart from MIR",
    file: "src/runtime.rs",
    find: "    \"$jsonIndex\",\n",
    replace: "",
    tests: ["test/serde.test.ts","-t","a Value's reads"],
  },
  {
    name: "range-step-may-throw",
    breaks: "a range's step, which never throws, is taken for a call that may, and a loop with a drop in it is a `while (true)` of a `try`, not a `for`",
    file: "src/runtime.rs",
    find: "    \"$rangeNext\",\n    \"$rangeNextBack\",\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "drop_flag_loops"],
    snapshots: true,
  },
  {
    name: "inclusive-range-step-may-throw",
    breaks: "an inclusive range's step, `$rangeInclusiveNext`, which never throws, is taken for a call that may, and a loop with a drop in it is a `while (true)` of a `try`, not a `for`",
    file: "src/runtime.rs",
    find: "    \"$rangeInclusiveNext\",\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^drop_reassigned_in_inner_loop.rs$"],
    snapshots: true,
  },
  {
    name: "some-value-not-pure",
    breaks: "taking a level off a nested `Option`, `$someValue(o)`, which only reads, is taken to do something, and the `Option` formatted after one is a function called in place",
    file: "src/runtime.rs",
    find: "    \"$someValue\",\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^iter_next_back.rs$"],
    snapshots: true,
  },
  {
    "name": "mut-get-impure",
    "breaks": "a handle on a map's value nothing reads is still made, `$mutGet(m, k);`",
    "file": "src/runtime.rs",
    "find": "    \"$mutGet\",\n",
    "replace": "",
    "tests": ["test/lowering-collections.test.ts", "-t", "and_modify then or_insert"]
  },
  {
    name: "some-mut-impure",
    breaks: "`e.get_or_insert(None);` of an `Option<Option<T>>` leaves its unread `$someMut({ .. })` as a statement",
    file: "src/runtime.rs",
    find: "    \"$someMut\",\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^boxed_option_places.rs$"],
    snapshots: true,
  },
];
