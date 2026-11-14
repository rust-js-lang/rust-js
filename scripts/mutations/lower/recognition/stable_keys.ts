// Mutations of src/lower/recognition/stable_keys.rs, whether a map's
// lookup of a key reads the same until the key is written (ADR 0059
// amended, #1844).
import type { Mutation } from "../../../mutations";

const file = "src/lower/recognition/stable_keys.rs";
const tests = ["test/corpus.test.ts", "-t", "^map_key_changed_between_test_and_index\\.rs$"];

export const mutations: Mutation[] = [
  {
    name: "key-stays-of-hand-written-impls",
    breaks: "a key whose hand-written `Hash` and `Eq` read a `Cell` or a static is taken to stay, so `m[k]` past a call that changes it is `m.get(k)`, `undefined` where Rust panics",
    file,
    find: "            if !derived || !impls(std_item(tcx, StdItem::Borrow)).is_empty() {\n",
    replace: "            if !impls(std_item(tcx, StdItem::Borrow)).is_empty() {\n",
    tests,
  },
  {
    name: "key-stays-of-any-type",
    breaks: "a key of any type, a type parameter's or a `dyn`'s, is taken to stay",
    file,
    find: "        _ => return false,\n    };\n",
    replace: "        _ => Vec::new(),\n    };\n",
    tests: ["test/corpus.test.ts", "-t", "^map_index_of_a_key_held\\.rs$|^map_key_changed_between_test_and_index\\.rs$"],
  },
];
