// Mutations of src/runtime/repeating_with.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "repeating-with-hint-unknown",
    breaks: "a `repeat_with`'s hint is none known, so an explicit `Take` of it hints `(0, Some(n))` where std's is `(n, Some(n))` (#1357's P0)",
    file: "src/runtime/repeating_with.js",
    find: "  it.sizeHint = () => [2 ** 32 - 1, undefined];\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^source_hint_where_an_adapter_holds_it.rs$"],
  },
];
