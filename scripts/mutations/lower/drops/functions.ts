// Mutations of src/lower/drops/functions.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "generic-drop-not-passed-on",
    breaks: "a generic function that passes its `T` to another doesn't pass its drop on",
    file: "src/lower/drops/functions.rs",
    find: "            return Ok(index.map(|index| Expr::var(&self.drop_state.param_drops[&index])));\n",
    replace: "            return Ok(None);\n",
    tests: ["test/corpus.test.ts", "-t", "drop_generic"],
  },
  {
    name: "drop-each-written-out",
    breaks: "what drops each item an iterator hasn't given is an arrow of a loop, not `$dropEach(drop)`",
    file: "src/lower/drops/functions.rs",
    find: "            return Ok(Some(Expr::call_helper(\"$dropEach\", vec![(**callee).clone()])));\n",
    replace: "            let _ = callee;\n",
    tests: ["test/corpus.test.ts", "-t", "^take_destructors.rs$"],
    snapshots: true,
  },
];
