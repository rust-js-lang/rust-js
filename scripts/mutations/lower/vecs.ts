// Mutations of src/lower/vecs.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "to-vec-shallow",
    breaks: "`to_vec()` of structs shares each with the original, which a change to the copy changes",
    file: "src/lower/vecs.rs",
    find: "                    Some(item) if self.thir[args[0]].ty.is_ref() => self.clone_items(items, item, span)?,",
    replace: "                    Some(item) if false && self.thir[args[0]].ty.is_ref() => self.clone_items(items, item, span)?,",
    tests: ["test/corpus.test.ts", "-t", "collection_methods"],
  },
];
