// Mutations of src/lower/recognition/held.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "box-iterator-not-held",
    breaks: "a `Box` of the crate's iterator isn't its iterator, so its value is stepped as plain data, `count()` of `Box::new(Up(0, 3))` its fields' (#1601)",
    file: "src/lower/recognition/held.rs",
    find: "    while let Some(inner) = ty.boxed_ty() {\n",
    replace: "    while let Some(inner) = ty.boxed_ty().filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_iterator_steps_as_std_box_does.rs$"],
  },
];
