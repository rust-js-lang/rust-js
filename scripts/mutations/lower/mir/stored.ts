// Mutations of src/lower/mir/stored.rs (ADR 0562, #1089, #1110, #1113).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "stored-items-unrecorded",
    breaks: "a borrow of a `Vec` records none of its items, so a field's address through its iterator's reference is its item's own allocation's, not `8` past the buffer's start (#1113)",
    file: "src/lower/mir/stored.rs",
    find: "        .filter(|&item| stored.records(item));\n",
    replace: "        .filter(|&item| stored.records(item) && false);\n",
    tests: ["test/corpus.test.ts", "-t", "^address_of_(a_field_where_its_object_is_stored|an_item_std_lends).rs$"],
  },
  {
    name: "stored-slice-item-recorded",
    breaks: "an item of a shared subslice, a copy of its items (ADR 0063), is recorded as stored in the copy, so `split_at`'s halves' items are another allocation's, `4104` where native Rust's is `8` (#1113)",
    file: "src/lower/mir/stored.rs",
    find: "from.variant_index.is_none() && !from.ty.is_slice() && stored.records(to.ty)",
    replace: "from.variant_index.is_none() && stored.records(to.ty)",
    tests: ["test/corpus.test.ts", "-t", "^address_of_an_item_std_lends.rs$"],
  },
  {
    name: "rooted-place-temporary-unmade",
    breaks: "a nested place whose root is what a `Deref` gives, `(*cell.borrow())[2].a`, reads the temporary holding it by its name, `_25`, which no JS declares: a `ReferenceError` (#1110)",
    file: "src/lower/mir/stored.rs",
    find: "        if pending {\n            self.flush(state, out)?;\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(address_of_a_nested_field_through_a_deref|pointer_into_a_borrow_read).rs$"],
    snapshots: true,
  },
  {
    name: "nested-place-unrooted",
    breaks: "a raw pointer to a place reached through more than one field or item is given no root, so its address is the inner object's own, 4096 from the root's where native Rust's is 8 (#1089)",
    file: "src/lower/mir/stored.rs",
    find: "        (place.projection.len() - start >= 2).then_some(start)\n",
    replace: "        (place.projection.len() - start >= 99).then_some(start)\n",
    tests: ["test/corpus.test.ts", "-t", "^address_of_a_nested_field.rs$"],
    snapshots: true,
  },
  {
    name: "nested-place-index-unscaled",
    breaks: "a nested place's index that's a variable's is its offset as it is, not times its item's size, `items[i].b` at `i` where native Rust's is `i * 8` (#1089)",
    file: "src/lower/mir/stored.rs",
    find: "                _ => scaled.push(Expr::bin(js::Op::Mul, index, Expr::int(i128::from(item)))),\n",
    replace: "                _ => scaled.push(index),\n",
    tests: ["test/corpus.test.ts", "-t", "^address_of_a_nested_field.rs$"],
    snapshots: true,
  },
  {
    name: "nested-place-field-offset-lost",
    breaks: "a nested place's field steps add no offset, `o.inner.a` at `o.inner`'s, where native Rust's is 4 past it (#1089)",
    file: "src/lower/mir/stored.rs",
    find: "                    bytes += layout.fields.offset(field.index()).bytes();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^address_of_a_nested_field.rs$"],
    snapshots: true,
  },
];
