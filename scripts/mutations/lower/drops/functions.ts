// Mutations of crates/rust-js-lower/src/drops/functions.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "held-into-iter-drop-of-its-of",
    breaks:
      "the drop of an `IntoIterator`'s `IntoIter` held as it is reads `.of` of the value `$given` gave as itself: `undefined is not iterable` (#966)",
    file: "crates/rust-js-lower/src/drops/functions.rs",
    find: "        if crate::recognition::known_by_bounds(ty) {\n",
    replace: "        if matches!(ty.kind(), ty::Param(_)) {\n",
    tests: ["test/corpus.test.ts", "-t", "^into_iterator_named_by_another_bound.rs$"],
  },
  {
    name: "generic-drop-not-passed-on",
    breaks:
      "a generic function that passes its `T` to another doesn't pass its drop on",
    file: "crates/rust-js-lower/src/drops/functions.rs",
    find: "            return Ok(self.use_given_drop(ty).map(|drop| (drop, true)));\n",
    replace: "            return Ok(None);\n",
    tests: ["test/corpus.test.ts", "-t", "drop_generic"],
  },
  {
    name: "drop-each-written-out",
    breaks:
      "what drops each item an iterator hasn't given, of what drops as an array's items, is an arrow of a loop, not `$dropEach(drop)`, so what a `Vec`'s `IntoIter` passes over from its back drops back to front, one by one, where std's `advance_back_by` drops it as one slice, front to back: of items with a `Drop` of their own whose drop may panic, a reversed `take(n).count()`'s, and of items whose drop is their drop glue, a struct's that holds a `D` (#670, #1854)",
    file: "crates/rust-js-lower/src/drops/functions.rs",
    find: "            && let Some((each, false)) = self.found_drop(item, None, span)?\n",
    replace: "            && let Some((each, false)) = self.found_drop(item, None, span)?.filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^(take_destructors|rev_take_count_drops_a_slice|glue_dropped_items_passed_from_a_vecs_back).rs$"],
    snapshots: true,
  },
  {
    name: "given-drop-called-as-there",
    breaks:
      "`slot.insert(v)` in generic code calls the drop its caller gives, `dropT(old)`, or an impl's dictionary holds, `IItemIntoIterator.$dropIntoIter(old)`, where it may be none: a TypeError where `T` has nothing to drop, std's `flatten` of a slice's iterators too",
    file: "crates/rust-js-lower/src/drops/functions.rs",
    find: "        Ok(Some(match given {\n",
    replace: "        Ok(Some(match given && false {\n",
    tests: ["test/corpus.test.ts", "-t", "^option_insert_given_drops.rs$"],
  },
  {
    name: "compiled-struct-dropped-as-made-of",
    breaks: "std's compiled struct a dictionary makes the JS iterator it is, `$knowing`'s, is dropped as what an `$iterator` was made of, `it.of`, which it hasn't: \"reading 'return'\" (#1296)",
    file: "crates/rust-js-lower/src/drops/functions.rs",
    find: "        if !iterated || !self.has_drops(ty) || crate::recognition::is_compiled_std_iterator(self.tcx, ty) {\n",
    replace: "        if !iterated || !self.has_drops(ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "^impl_trait_of_a_trait_method_dropped_by_its_dictionary.rs$"],
  },
];
