// Mutations of src/lower/iterators/clones.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "in-place-part-cloned-held",
    breaks:
      "a `Map` made in place, part of a `filter` cloned apart, is cloned by std's compiled `Clone` of a struct it isn't",
    file: "src/lower/iterators/clones.rs",
    find: '        match self.iterator_cloned_apart(ty) {\n            true => Ok(Expr::call_helper("$clone", vec![place])),\n',
    replace:
      '        match false {\n            true => Ok(Expr::call_helper("$clone", vec![place])),\n',
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(std_map|lazy_iterator_clone|cloned_chains_and_cycles).rs$",
    ],
  },
  {
    name: "in-place-part-copied-held",
    breaks:
      "a `copied()` of a `Map` made in place, part of a `filter` cloned apart, clones the `Map` as std's struct",
    file: "src/lower/iterators/clones.rs",
    find: "            return self.clone_in_place(place, source, span, out);\n",
    replace: "            return self.clone_value(place, source, span, out);\n",
    tests: ["test/corpus.test.ts", "-t", "^(lazy_iterator_clone).rs$"],
  },
  {
    name: "copied-own-clone-spread",
    breaks:
      "a clone of a `copied()` of the crate's own iterator clones the JS iterator made of it as the iterator's struct, `{ ...copied }`, whose items `Array.from(again)` reads none of, an empty array (#384)",
    file: "src/lower/iterators/clones.rs",
    find: '        let of = self.clone_value(Expr::member(place.clone(), "of"), source, span, out)?;\n',
    replace:
      '        return self.clone_value(place, source, span, out);\n        let of = self.clone_value(Expr::member(place.clone(), "of"), source, span, out)?;\n',
    tests: ["test/corpus.test.ts", "-t", "^copied_clone_of_its_own.rs$"],
  },
  {
    name: "copied-given-clone-by-dictionary",
    breaks:
      "a clone of generic code's `copied()` of what it's given is its `Clone` dictionary's of the JS iterator `$given` made, `(5, 0)` where Rust gives `(5, 3)` (#384)",
    file: "src/lower/iterators/clones.rs",
    find: '            return Ok(Expr::call_helper("$givenClone", vec![place, clone]));\n',
    replace: "            return Ok(Expr::call(clone, vec![place]));\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_clone_of_its_own.rs$"],
  },
  {
    name: "copied-dropped-clone-allowed",
    breaks:
      "a clone of a `copied()` of the crate's iterator with a destructor is made of the `$closing` JS iterator, which throws (#384)",
    file: "src/lower/iterators/clones.rs",
    find: "        if dropped {\n",
    replace: "        if dropped && false {\n",
    tests: [
      "test/limitations.test.ts",
      "-t",
      "A clone of a .copied... of an iterator dropped",
    ],
  },
  {
    name: "copied-copying-clone-allowed",
    breaks:
      "a clone of a `copied()` of items something changes in place, `source.map(copy)`, is cloned as its source, which throws (#384)",
    file: "src/lower/iterators/clones.rs",
    find: "        if !self.is_its_source(ty) {\n",
    replace: "        if !self.is_its_source(ty) && false {\n",
    tests: [
      "test/limitations.test.ts",
      "-t",
      "A clone of a .copied... whose items",
    ],
  },
];
