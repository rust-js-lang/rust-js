// Mutations of crates/rust-js-lower/src/traits/dictionaries/dyn_objects.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "object-mut-receiver-unboxed",
    breaks:
      "a `&mut self` method of a `dyn` in generic code takes generic code's box for the pair, and JS throws",
    file: "crates/rust-js-lower/src/traits/dictionaries/dyn_objects.rs",
    find: "            let pair = match mutable {\n                true => Expr::held(object.clone()),",
    replace:
      "            let pair = match mutable {\n                true => object.clone(),",
    tests: ["test/corpus.test.ts", "-t", "dyn_generic"],
  },
  {
    name: "object-supertraits-missing",
    breaks:
      "a `dyn`'s dictionary in generic code has no supertraits, and a supertrait's method throws",
    file: "crates/rust-js-lower/src/traits/dictionaries/dyn_objects.rs",
    find: "            .ok_or_else(|| self.no_evidence(span, tr))?;\n        let mut props = self.supertrait_entries(tr, span)?;\n",
    replace:
      "            .ok_or_else(|| self.no_evidence(span, tr))?;\n        let mut props = Vec::new();\n",
    tests: ["test/corpus.test.ts", "-t", "dyn_generic"],
  },
  {
    name: "dyn-item-drops-left-out",
    breaks:
      "a `dyn`'s dictionary has no drop for the associated type its type names, so generic code given one drops nothing",
    file: "crates/rust-js-lower/src/traits/dictionaries/dyn_objects.rs",
    find: "        }\n        props.extend(self.item_drops(tr, false, span)?);\n        Ok(Expr::object(props))\n",
    replace: "        }\n        Ok(Expr::object(props))\n",
    tests: ["test/corpus.test.ts", "-t", "associated_type_drops"],
  },
  {
    name: "dyn-shim-params-unnamed",
    breaks: "a `dyn`'s dictionary's entry names its parameters `arg1`, not as the trait's method does, `(object, i)` of `write_i8(&mut self, i)` (#1328)",
    file: "crates/rust-js-lower/src/traits/dictionaries/dyn_objects.rs",
    find: "                let named = names.get(i).cloned().flatten()",
    replace: "                let named = names.get(usize::MAX).cloned().flatten()",
    tests: ["test/corpus.test.ts", "-t", "^hasher_written_through_a_box.rs$"],
    snapshots: true,
  },
];
