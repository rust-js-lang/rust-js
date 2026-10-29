// Mutations of src/lower/recognition/owners.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "owned-iterator-item-first-arg",
    breaks: "a `Drain`'s items' type is read as its lifetime, and crashes",
    file: "src/lower/recognition/owners.rs",
    find: "            return self.iterator_item(args.types().next()?);\n        }\n        args.types().next()\n",
    replace: "            return self.iterator_item(args.types().next()?);\n        }\n        Some(args.type_at(0))\n",
    tests: ["test/corpus.test.ts", "-t", "^drain_destructors.rs$"],
  },
  {
    name: "map-drain-items-keys",
    breaks: "a map's `drain()` is taken to own its keys alone, and drops none of its values",
    file: "src/lower/recognition/owners.rs",
    find: "            \"std::collections::hash_map::Drain\"\n                | \"std::collections::hash_map::IntoIter\"\n",
    replace: "            \"std::collections::hash_map::IntoIter\"\n",
    tests: ["test/corpus.test.ts", "-t", "^map_value_destructors.rs$"],
  },
  {
    name: "splice-items-its-iterator",
    breaks: "a `splice`'s items are taken for its replacing iterator, and dropped as one",
    file: "src/lower/recognition/owners.rs",
    find: "        if std_path(self.tcx, adt.did()) == \"std::vec::Splice\" {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^splice_and_inspect_drops.rs$"],
  },
  {
    name: "map-into-iter-items-its-keys",
    breaks: "a map's `into_iter()` owns its keys alone, so what a loop leaves of its values isn't dropped",
    file: "src/lower/recognition/owners.rs",
    find: "                | \"std::collections::hash_map::IntoIter\"\n                | \"std::collections::btree_map::IntoIter\"\n        ) {",
    replace: "        ) {",
    tests: ["test/corpus.test.ts", "-t", "^map_into_iter_drops.rs$"],
  },
];
