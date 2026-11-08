// Mutations of src/lower/std_types/btree.rs (#926, ADRs 0325, 0404).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "tree-extract-checked",
    breaks:
      "a B-tree's `extract_if` of a range that ends before it starts panics",
    file: "src/lower/std_types/btree.rs",
    find: "        } else if !checked {\n            list.push(Expr::bool(false));\n",
    replace: "        } else if !checked {\n",
    tests: ["test/corpus.test.ts", "-t", "extract_if"],
  },
  {
    name: "map-extract-no-handles",
    breaks:
      "a map's `extract_if` gives its filter a number, not a handle on it",
    file: "src/lower/std_types/btree.rs",
    find: "        let handles = !set\n            && matches!(",
    replace: "        let handles = false\n            && matches!(",
    tests: ["test/corpus.test.ts", "-t", "extract_if"],
  },
  {
    name: "range-mut-no-handles",
    breaks: "a B-tree's `range_mut` of numbers gives the numbers",
    file: "src/lower/std_types/btree.rs",
    find: "        let handles = mutable && self.handles_mut(map.type_at(1));",
    replace: "        let handles = false;",
    tests: ["test/corpus.test.ts", "-t", "btree_range_mut"],
  },
  {
    name: "range-mut-handles-objects",
    breaks: "a B-tree's `range_mut` of tuples gives handles on them",
    file: "src/lower/std_types/btree.rs",
    find: "        let handles = mutable && self.handles_mut(map.type_at(1));",
    replace: "        let handles = mutable;",
    tests: ["test/corpus.test.ts", "-t", "btree_range_mut"],
  },
  {
    name: "tree-of-nodes-never",
    breaks: "a B-tree of keys whose order can be seen is a sorted array, whose halving search asks other keys than std's nodes (#926)",
    file: "src/lower/std_types/btree.rs",
    find: "        self.found_by_cmp(key) && (self.order_seen(key) || self.eq_seen(key) || !self.order_total(key))\n",
    replace: "        self.found_by_cmp(key) && (self.eq_seen(key) || !self.order_total(key))\n",
    tests: ["test/corpus.test.ts", "-t", "^(btree_nodes_seen|own_ne_max_min_clamp).rs$"],
  },
  {
    name: "tree-of-nodes-not-by-ne",
    breaks: "a B-tree of keys whose `!=` can be seen is a sorted array, whose bulk build never asks `!=` of each two in a row, as std's does (#962)",
    file: "src/lower/std_types/btree.rs",
    find: "        self.found_by_cmp(key) && (self.order_seen(key) || self.eq_seen(key) || !self.order_total(key))\n",
    replace: "        self.found_by_cmp(key) && (self.order_seen(key) || !self.order_total(key))\n",
    tests: ["test/corpus.test.ts", "-t", "^own_ne_max_min_clamp.rs$"],
  },
  {
    name: "extract-if-holds-nothing-to-drop",
    breaks: "a map's or a set's `extract_if` is given nothing to drop as it's closed, so its test and a B-tree's owned bounds are never dropped (#945)",
    file: "src/lower/std_types/btree.rs",
    find: "            false => Some(Expr::arrow(Vec::new(), self.drop_each(held, span)?)),\n",
    replace: "            false => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^extract_if_drops_what_it_holds.rs$"],
  },
];
