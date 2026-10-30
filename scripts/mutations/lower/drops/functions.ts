// Mutations of src/lower/drops/functions.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "generic-drop-not-passed-on",
    breaks: "a generic function that passes its `T` to another doesn't pass its drop on",
    file: "src/lower/drops/functions.rs",
    find: "            return Ok(index.map(|index| (Expr::var(&self.drop_state.param_drops[&index]), true)));\n",
    replace: "            return Ok(None);\n",
    tests: ["test/corpus.test.ts", "-t", "drop_generic"],
  },
  {
    name: "drop-each-written-out",
    breaks: "what drops each item an iterator hasn't given is an arrow of a loop, not `$dropEach(drop)`",
    file: "src/lower/drops/functions.rs",
    find: "            return Ok(Some((Expr::call_helper(\"$dropEach\", vec![(**callee).clone()]), false)));\n",
    replace: "            let _ = callee;\n",
    tests: ["test/corpus.test.ts", "-t", "^take_destructors.rs$"],
    snapshots: true,
  },
  {
    name: "given-drop-called-as-there",
    breaks: "`slot.insert(v)` in generic code calls the drop its caller gives, `dropT(old)`, or an impl's dictionary holds, `IItemIntoIterator.$dropIntoIter(old)`, where it may be none: a TypeError where `T` has nothing to drop, std's `flatten` of a slice's iterators too",
    file: "src/lower/drops/functions.rs",
    find: "        Ok(Some(match given {\n",
    replace: "        Ok(Some(match given && false {\n",
    tests: ["test/corpus.test.ts", "-t", "^option_insert_given_drops.rs$"],
  },
];
