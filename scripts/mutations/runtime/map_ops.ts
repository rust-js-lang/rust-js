// Mutations of src/runtime/map_ops.js (ADR 0325).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "retain-map-keeps-all",
    breaks: "a map's `retain` keeps what `f` rejects",
    file: "src/runtime/map_ops.js",
    find: "    if (!f(key, value)) m.delete(key);\n",
    replace: "    f(key, value);\n",
    tests: ["test/corpus.test.ts", "-t", "map_set_methods"],
  },
  {
    name: "tree-end-reversed",
    breaks: "a B-tree's `first` is its last",
    file: "src/runtime/map_ops.js",
    find: "    if (!seen || (last ? order > 0 : order < 0)) {\n",
    replace: "    if (!seen || (last ? order < 0 : order > 0)) {\n",
    tests: ["test/corpus.test.ts", "-t", "map_set_methods"],
  },
  {
    name: "tree-range-end-included",
    breaks: "a B-tree's `a..b` range takes `b` too",
    file: "src/runtime/map_ops.js",
    find: "    if (hasEnd && (endIncluded ? cmp(key, end) > 0 : cmp(key, end) >= 0)) return false;\n",
    replace: "    if (hasEnd && cmp(key, end) > 0) return false;\n",
    tests: ["test/corpus.test.ts", "-t", "map_set_methods"],
  },
  {
    name: "tree-range-unchecked",
    breaks: "a B-tree's range starting past its end is empty, not std's panic",
    file: "src/runtime/map_ops.js",
    find: "  if (checked && hasStart && hasEnd && cmp(start, end) > 0) {\n",
    replace: "  if (false) {\n",
    tests: ["test/corpus.test.ts", "-t", "btree_range_panic"],
  },
  {
    name: "tree-split-off-past-key",
    breaks: "`split_off(&k)` leaves `k` behind",
    file: "src/runtime/map_ops.js",
    find: "    if (cmp(k, key) >= 0) {\n",
    replace: "    if (cmp(k, key) > 0) {\n",
    tests: ["test/corpus.test.ts", "-t", "map_set_methods"],
  },
  {
    name: "tree-append-keeps-other",
    breaks: "`append` leaves the other map full",
    file: "src/runtime/map_ops.js",
    find: "  other.clear();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "map_set_methods"],
  },
  {
    name: "set-algebra-unsorted",
    breaks: "a B-tree set's `union` isn't in order",
    file: "src/runtime/map_ops.js",
    find: "  return cmp ? items.sort(cmp) : items;\n",
    replace: "  return items;\n",
    tests: ["test/corpus.test.ts", "-t", "map_set_methods"],
  },
  {
    name: "is-subset-superset-same",
    breaks: "`is_superset` asks `is_subset`",
    file: "src/runtime/map_ops.js",
    find: "  const [inner, outer] = superset ? [b, a] : [a, b];\n",
    replace: "  const [inner, outer] = [a, b];\n",
    tests: ["test/corpus.test.ts", "-t", "map_set_methods"],
  },
];
