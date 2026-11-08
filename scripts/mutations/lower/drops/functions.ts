// Mutations of src/lower/drops/functions.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "held-into-iter-drop-of-its-of",
    breaks:
      "the drop of an `IntoIterator`'s `IntoIter` held as it is reads `.of` of the value `$given` gave as itself: `undefined is not iterable` (#966)",
    file: "src/lower/drops/functions.rs",
    find: "        if crate::lower::recognition::known_by_bounds(ty) {\n",
    replace: "        if matches!(ty.kind(), ty::Param(_)) {\n",
    tests: ["test/corpus.test.ts", "-t", "^into_iterator_named_by_another_bound.rs$"],
  },
  {
    name: "generic-drop-not-passed-on",
    breaks:
      "a generic function that passes its `T` to another doesn't pass its drop on",
    file: "src/lower/drops/functions.rs",
    find: "            return Ok(self.use_given_drop(ty).map(|drop| (drop, true)));\n",
    replace: "            return Ok(None);\n",
    tests: ["test/corpus.test.ts", "-t", "drop_generic"],
  },
  {
    name: "drop-each-written-out",
    breaks:
      "what drops each item an iterator hasn't given is an arrow of a loop, not `$dropEach(drop)`",
    file: "src/lower/drops/functions.rs",
    find: '            return Ok(Some((Expr::call_helper("$dropEach", vec![(**callee).clone()]), false)));\n',
    replace: "            let _ = callee;\n",
    tests: ["test/corpus.test.ts", "-t", "^take_destructors.rs$"],
    snapshots: true,
  },
  {
    name: "given-drop-called-as-there",
    breaks:
      "`slot.insert(v)` in generic code calls the drop its caller gives, `dropT(old)`, or an impl's dictionary holds, `IItemIntoIterator.$dropIntoIter(old)`, where it may be none: a TypeError where `T` has nothing to drop, std's `flatten` of a slice's iterators too",
    file: "src/lower/drops/functions.rs",
    find: "        Ok(Some(match given {\n",
    replace: "        Ok(Some(match given && false {\n",
    tests: ["test/corpus.test.ts", "-t", "^option_insert_given_drops.rs$"],
  },
  {
    name: "drop-each-of-a-throwing-drop-unseen",
    breaks: "an iterator's drop of items whose drop may panic, `$dropItems(intoIter, dDrop_drop)`, isn't `$dropEach(dDrop_drop)`, so a reversed `Vec`'s `take(n).count()` drops what it passes back to front, one by one (#670)",
    file: "src/lower/drops/functions.rs",
    find: "            && helper.helper_name() == Some(\"$dropItems\")\n",
    replace: "            && helper.helper_name() == Some(\"$dropItemz\")\n",
    tests: ["test/corpus.test.ts", "-t", "^rev_take_count_drops_a_slice.rs$"],
  },
];
