// Mutations of src/runtime/cow_mut.js (ADR 0319).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "cow-mut-stays-borrowed",
    breaks: "a `Cow` of text `to_mut()` made owned still says it's borrowed",
    file: "src/runtime/cow_mut.js",
    find: '  cow.TAG = "Owned";\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^cows"],
  },
];
