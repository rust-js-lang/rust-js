// Mutations of src/runtime/held.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "held-unit-value-read",
    breaks: "a unit struct's iterator, `undefined`, a library's function makes by its `IntoIterator` dictionary and holds, is asked its `hold`, which throws (itertools' `zip_eq`)",
    file: "src/runtime/held.js",
    find: "  it?.hold?.();\n",
    replace: "  it.hold?.();\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_default_takes_an_iterator_as_its_callers_give_it"],
  },
];
