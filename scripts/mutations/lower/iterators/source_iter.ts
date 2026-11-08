// Mutations of src/lower/iterators/source_iter.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "source-iter-inner-undropped",
    breaks: "std's compiled structs are made with no `dropInner`, so a collect in place drops nothing of its source's rest under its guard (#852)",
    file: "src/lower/iterators/source_iter.rs",
    find: "        if !drop.is_empty() {\n",
    replace: "        if false {\n",
    tests: ["test/std-package.test.ts", "-t", "committed @rust-js/std is std's bodies"],
  },
];
