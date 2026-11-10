// Mutations of src/lower/recognition/owners.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "owned-iterator-item-first-arg",
    breaks: "a `Drain`'s items' type is read as its lifetime, and crashes",
    file: "src/lower/recognition/owners.rs",
    find: "            return args.types().nth(1);\n        }\n        args.types().next()\n",
    replace: "            return args.types().nth(1);\n        }\n        Some(args.type_at(0))\n",
    tests: ["test/corpus.test.ts", "-t", "^drain_destructors.rs$"],
  },
  {
    name: "map-drain-items-keys",
    breaks:
      "a map's `drain()` is taken to own its keys alone, and drops none of its values",
    file: "src/lower/recognition/owners.rs",
    find: '            "std::collections::hash_map::Drain"\n                | "std::collections::hash_map::IntoIter"\n',
    replace: '            "std::collections::hash_map::IntoIter"\n',
    tests: ["test/corpus.test.ts", "-t", "^map_value_destructors.rs$"],
  },
  {
    name: "map-into-iter-items-its-keys",
    breaks:
      "a map's `into_iter()` owns its keys alone, so what a loop leaves of its values isn't dropped",
    file: "src/lower/recognition/owners.rs",
    find: '                | "std::collections::hash_map::IntoIter"\n                | "std::collections::btree_map::IntoIter"\n        ) {',
    replace: "        ) {",
    tests: ["test/corpus.test.ts", "-t", "^map_into_iter_drops.rs$"],
  },
  {
    name: "into-values-owns-keys",
    breaks:
      "a map's `into_values()` owns its keys, `K`, so what a `take` leaves of its values isn't dropped (#432)",
    file: "src/lower/recognition/owners.rs",
    find: "            return args.types().nth(1);\n",
    replace: "            return args.types().next();\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^collection_taken_drops_what_is_left.rs$",
    ],
  },
  {
    name: "splice-of-a-vec-lazy",
    breaks: "a `Vec` given whole as a `splice`'s replacement isn't an array's items, so its `Splice` is the lazy one where nothing runs (#1189)",
    file: "src/lower/recognition/owners.rs",
    find: "                || matches!(it.kind(), ty::Adt(adt, _) if std_path(self.tcx, adt.did()) == \"std::vec::IntoIter\")\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^splice_of_array_items.rs$"],
    snapshots: true,
  },
  {
    name: "splice-of-copies-lazy",
    breaks: "a slice's `copied()` given as a `splice`'s replacement isn't an array's items, so its `Splice` is the lazy one where nothing runs (#1189)",
    file: "src/lower/recognition/owners.rs",
    find: "        (array(replacement) || copied)\n            .then(|| self.iterator_item(replacement))\n            .flatten()\n",
    replace: "        array(replacement)\n            .then(|| self.iterator_item(replacement))\n            .flatten()\n",
    tests: ["test/corpus.test.ts", "-t", "^splice_of_array_items.rs$"],
    snapshots: true,
  },
];
