// Mutations of src/lower/std_types/vec.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "to-vec-shallow",
    breaks: "`to_vec()` of structs shares each with the original, which a change to the copy changes",
    file: "src/lower/std_types/vec.rs",
    find: "                    Some(item) if self.thir[args[0]].ty.is_ref() => self.clone_items(items, item, span)?,",
    replace: "                    Some(item) if false && self.thir[args[0]].ty.is_ref() => self.clone_items(items, item, span)?,",
    tests: ["test/corpus.test.ts", "-t", "collection_methods"],
  },
  {
    name: "stepping-count-length",
    breaks: "`count()` of an iterator that knows where it is is `undefined`",
    file: "src/lower/std_types/vec.rs",
    find: "            Std::Len if self.is_stepping(args[0]) => {",
    replace: "            Std::Len if false => {",
    tests: ["test/corpus.test.ts", "-t", "iter_by_ref"],
  },
  {
    name: "text-empty-by-length",
    breaks: "a string's `is_empty()` is `text.length === 0`, not `!text`",
    file: "src/lower/std_types/vec.rs",
    find: "            Std::IsEmpty if self.is_string_like(self.thir[args[0]].ty.peel_refs()) => {\n",
    replace: "            Std::IsEmpty if false && self.is_string_like(self.thir[args[0]].ty.peel_refs()) => {\n",
    tests: ["test/jsx.test.ts", "-t", "filtered child, in place"],
  },
  {
    name: "lazy-count-length",
    breaks: "`count()` of a JS iterator, `list.values().filter(..)`, is its `.length`, which a JS iterator doesn't have",
    file: "src/lower/std_types/vec.rs",
    find: "            Std::Len if self.is_lazy_value(args[0]) => {",
    replace: "            Std::Len if false && self.is_lazy_value(args[0]) => {",
    tests: ["test/compiler.test.ts", "-t", "the webapi crate's bindings become plain JS"],
  },
];
