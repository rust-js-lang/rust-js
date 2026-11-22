// Mutations of crates/rust-js-lower/src/traits/dictionaries/placed.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "placed-entry-unread",
    breaks:
      "a placed dictionary's entry that takes a fieldless enum's iterator by `&` is given the place, its `size_hint` printing `[object Object]` (#2255)",
    file: "crates/rust-js-lower/src/traits/dictionaries/placed.rs",
    find: "            shaped_entry(&name, holder, (&mut given, &mut made));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^non_object_iterator_given_to_generic_code\\.rs$"],
  },
  {
    name: "placed-entry-unmade",
    breaks:
      "a placed dictionary's `clone()` gives the clone bare, so generic code steps the bare string (#2255)",
    file: "crates/rust-js-lower/src/traits/dictionaries/placed.rs",
    find: "    if let Some(gives) = gives {\n",
    replace: "    if let Some(gives) = gives.filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^non_object_iterator_given_to_generic_code\\.rs$"],
  },
  {
    name: "placed-trait-box-unread",
    breaks:
      "a `&mut` to a placed iterator a trait declares as a `&mut T` it boxes, `Lend::lend`'s, is given generic code as the bare value it reads out of the box (#2255)",
    file: "crates/rust-js-lower/src/traits/dictionaries/placed.rs",
    find: "                    true => Some(Placed::Itself),\n",
    replace: "                    true => self.placed_shape((def_id, d, i)),\n",
    tests: ["test/corpus.test.ts", "-t", "^non_object_iterator_given_to_generic_code\\.rs$"],
  },
  {
    name: "placed-parts-unplaced",
    breaks:
      "an `IntoIterator`'s `IntoIter` `Iterator` entry of a placed iterator isn't placed, so std's `Zip` steps it with its own `size_hint` given the place (#2255)",
    file: "crates/rust-js-lower/src/traits/dictionaries/placed.rs",
    find: "            if let Some(part) = self.placed_dictionary(def_id, (of(theirs, there), of(ours, self.typing_env)), span)? {\n",
    replace: "            if let Some(part) = None::<Expr> {\n",
    tests: ["test/corpus.test.ts", "-t", "^non_object_iterator_given_to_generic_code\\.rs$"],
  },
  {
    name: "placed-entry-into-unmade",
    breaks:
      "a tuple's own impl of a trait, whose generic code holds each part in a place, is given its dictionary's tuple as its caller holds it, so its `step` stores into the bare string, \"Cannot create property 'value' on string\" (#2255)",
    file: "crates/rust-js-lower/src/traits/dictionaries/placed.rs",
    find: "            shaped_entry(&name, entry, (&mut into, &mut back));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^tuple_of_placed_iterators_crosses_part_by_part\\.rs$"],
  },
  {
    name: "placed-entry-alike-adapted",
    breaks:
      "a blanket impl's dictionary, whose default method holds the iterator in a place as the generic code holding the dictionary does, is read out of its place and placed again, `$intoPlace($throughPlace(..))`, where it's given as it is (#2255)",
    file: "crates/rust-js-lower/src/traits/dictionaries/placed.rs",
    find: "                if holder == entry {\n",
    replace: "                if false {\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^tuple_of_placed_iterators_crosses_part_by_part\\.rs$"],
  },
  {
    name: "placed-conversion-entry-placed-twice",
    breaks:
      "a conversion's dictionary, whose entry gives its `From` impl the tuple in places itself, is wrapped in `$intoPlace` too, so itertools' `multizip` shape gives the impl a place of a place and steps `{ value: \"Red\" }` (#2255)",
    file: "crates/rust-js-lower/src/traits/dictionaries/placed.rs",
    find: "                true => None,\n",
    replace: "                true => self.call_callee(method, args_of(bound))?,\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_holder_of_a_placed_iterator_built_by_its_generic_code"],
  },
  {
    name: "placed-dictionary-words-made-first",
    breaks: "a placed dictionary's refusal words are made before it's known to refuse, printing the trait's path and its type in every compile that holds an iterator in a place of its own, a walk of every crate's modules, about 37 ms (#2321)",
    file: "crates/rust-js-lower/src/traits/dictionaries/placed.rs",
    find: "                bound.self_ty()\n            )\n        };\n",
    replace: "                bound.self_ty()\n            )\n        };\n        let _ = what();\n",
    tests: ["test/phase-times.test.ts", "-t", "a lowering forces no walk of every crate"],
  },
];
