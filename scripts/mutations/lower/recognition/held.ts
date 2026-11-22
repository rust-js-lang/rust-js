// Mutations of crates/rust-js-lower/src/recognition/held.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "held-unboxed-peels-nothing",
    breaks:
      "a sized box of an iterator is taken for a type of its own, not what it holds: generic code's `&mut Box<I>` is given the crate's value raw and never stepped, its hint unasked, a box closed field by field (#2219, #2221)",
    file: "crates/rust-js-lower/src/recognition/held.rs",
    find: "        ty = inner;\n    }\n    ty\n}\n",
    replace: "        let _ = inner;\n        break;\n    }\n    ty\n}\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_box_of_an_iterator_is_what_it_holds.rs$"],
  },
  {
    name: "box-iterator-not-held",
    breaks: "a `Box` of the crate's iterator isn't its iterator, so its value is stepped as plain data, `count()` of `Box::new(Up(0, 3))` its fields' (#1601)",
    file: "crates/rust-js-lower/src/recognition/held.rs",
    find: "    while let Some(inner) = ty.boxed_ty() {\n",
    replace: "    while let Some(inner) = ty.boxed_ty().filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_iterator_steps_as_std_box_does.rs$"],
  },
];
