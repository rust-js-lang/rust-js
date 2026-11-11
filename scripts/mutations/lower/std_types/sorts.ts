// Mutations of src/lower/std_types/sorts.rs, split from src/lower/std_types/slice.rs by concept.
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "sort-kind-small-general",
    breaks:
      "an item of 32 bytes takes the fallback small sort, not the general one",
    file: "src/lower/std_types/sorts.rs",
    find: "        } else if freeze && size * 48 <= 4096 {",
    replace: "        } else if freeze && size * 48 <= 1024 {",
    tests: ["test/corpus.test.ts", "-t", "sort_unstable_ties"],
  },
  {
    name: "compare-inline-le",
    breaks: "a comparator's `is_less` is `<= 0`",
    file: "src/lower/std_types/sorts.rs",
    find: "                    let less = Expr::bin(Op::Lt, order.clone(), Expr::int(0));",
    replace:
      "                    let less = Expr::bin(Op::Le, order.clone(), Expr::int(0));",
    tests: ["test/corpus.test.ts", "-t", "sort_unstable_ties"],
  },
  {
    name: "select-no-handle",
    breaks: "`select_nth_unstable`'s number isn't a handle",
    file: "src/lower/std_types/sorts.rs",
    find: "        if select && self.handles_mut(item) {",
    replace: "        if false {",
    tests: ["test/corpus.test.ts", "-t", "sort_unstable_ties"],
  },
  {
    name: "sort-kind-host-size",
    breaks:
      "an unstable sort picks by a 64-bit host's sizes, not rust-js's 32-bit ones",
    file: "src/lower/std_types/sorts.rs",
    find: "        let size = layout.size.bytes();\n",
    replace: "        let size = layout.size.bytes() * 2;\n",
    tests: ["test/corpus.test.ts", "-t", "sort_unstable_wasm32"],
  },
  {
    name: "sort-of-held-param-measured",
    breaks:
      "an unstable sort of items that hold a type parameter asks rustc their layout, which it can't give",
    file: "src/lower/std_types/sorts.rs",
    find: '        if item.has_param() {\n            return Err(self.unsupported(\n                span,\n                "an unstable sort of items that hold a type parameter, whose order std\'s algorithm picks by their layout",\n',
    replace:
      '        if false {\n            return Err(self.unsupported(\n                span,\n                "an unstable sort of items that hold a type parameter, whose order std\'s algorithm picks by their layout",\n',
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "an unstable sort of items that hold a type parameter",
    ],
  },
  {
    name: "utf16-sort-unstable-kept",
    breaks:
      "`sort_unstable_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()))` is `$sortUnstable`, where JS's `sort()` is that order, its ties the same string (ADR 0945)",
    file: "src/lower/std_types/sorts.rs",
    find: "        if matches!(op, SliceOp::SortUnstable { .. })\n",
    replace: "        if false\n",
    tests: ["test/corpus.test.ts", "-t", "^utf16_sort.rs$"],
    snapshots: true,
  },
  {
    name: "stable-kind-sized-as-general8",
    breaks: "a `Freeze` item of 24 bytes is small-sorted eight at a time, where std's sort of one of more than 16 does four (#925)",
    file: "src/lower/std_types/sorts.rs",
    find: "            true if size <= 16 => \"general8\",",
    replace: "            true if size <= 32 => \"general8\",",
    tests: ["test/corpus.test.ts", "-t", "^sort_stable_seen.rs$"],
  },
  {
    name: "stable-kind-freeze-unasked",
    breaks: "an item that isn't `Freeze`, of a `Cell`, is small-sorted as a `Freeze` one, where std's insertion sort stops at 16 (#925)",
    file: "src/lower/std_types/sorts.rs",
    find: "        match item.is_freeze(self.tcx, self.typing_env) {",
    replace: "        match true {",
    tests: ["test/corpus.test.ts", "-t", "^sort_stable_seen.rs$"],
  },
  {
    name: "stable-key-order-unasked",
    breaks: "a quiet key of an order that counts is JS's sort, asking another sequence of the keys' comparisons (#925)",
    file: "src/lower/std_types/sorts.rs",
    find: "                let order = self.order_seen(key) || !self.order_total(key);\n",
    replace: "                let order = !self.order_total(key);\n",
    tests: ["test/corpus.test.ts", "-t", "^sort_stable_seen.rs$"],
  },
];
