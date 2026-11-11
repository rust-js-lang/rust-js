// Mutations of src/lower/drops/ref_cells/places.rs (ADR 0093), split from ref_cells.ts's list with its code (#1334).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "plain-refcell-any-borrow",
    breaks:
      'a field\'s `RefCell` borrowed, `fiber.r#return.borrow()`, is taken as lent, its every borrow counted on `$fieldCell(fiber, "return", true)`',
    file: "src/lower/drops/ref_cells/places.rs",
    find: "            Std::Borrow { lock: false, .. }\n                | Std::TryBorrow { lock: false, .. }\n                | Std::CellReplace\n",
    replace:
      "            Std::TryBorrow { lock: false, .. }\n                | Std::CellReplace\n",
    tests: [
      "test/lowering-field-cells.test.ts",
      "-t",
      "whatever else has its type",
    ],
  },
  {
    name: "replace-lends-the-cell",
    breaks:
      "a field's `RefCell` a `replace` writes is taken as lent, its borrows counted on its one cell, React's fiber's `return` `$fieldCell(fiber, \"return\", true)`",
    file: "src/lower/drops/ref_cells/places.rs",
    find: "                | Std::CellReplace\n                | Std::CellTake\n",
    replace: "                | Std::CellTake\n",
    tests: [
      "test/lowering-field-cells.test.ts",
      "-t",
      "whatever else has its type",
    ],
  },
  {
    name: "get-mut-lends-the-cell",
    breaks:
      "a field's `RefCell` written through `get_mut` is taken as lent, React's `Transition`'s `types` no longer one its contract says is quiet (ADR 0863)",
    file: "src/lower/drops/ref_cells/places.rs",
    find: "                | Std::CellGetMut\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "a library's RefCells are the values"],
  },
  {
    name: "thread-local-key-a-value",
    breaks:
      "a thread-local's key, `VALUE_STACK` of `VALUE_STACK.with_borrow_mut(..)`, is taken for a value of its cell's type, and the crate refused as its storage was (ADR 0863, amended)",
    file: "src/lower/drops/ref_cells/places.rs",
    find: "            walk.skip_current_subtree();\n            continue;\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: thread_local_cell"],
  },
  {
    name: "thread-local-init-a-value",
    breaks:
      "what a thread-local's `init` gives, its cell, is taken for a value of its type, `RefCell::new(vec![..])`, and the crate refused (ADR 0863, amended)",
    file: "src/lower/drops/ref_cells/places.rs",
    find: "        made.push(body.expr);\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: thread_local_cell"],
  },
  {
    name: "field-made-of-any-value",
    breaks:
      "a generic struct's `RefCell<T>` field made where the struct is, `Cursor { current: RefCell::new(value) }`, is a `RefCell<T>` of any value, which boxes every `RefCell` of an object, `{ value: [4, 5] }` (ADR 0863, amended)",
    file: "src/lower/drops/ref_cells/places.rs",
    find: "                    made.push(field.expr);\n",
    replace: "                    let _ = field;\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_generic_field_made.rs$"],
    snapshots: true,
  },
  {
    name: "variant-made-a-value",
    breaks: "a generic variant's `RefCell<T>` field made where the variant is, `Slot::Full { current: RefCell::new(value) }`, is a `RefCell<T>` of any value, which boxes every `RefCell` of an object, `{ value: [4, 5] }` (#503)",
    file: "src/lower/drops/ref_cells/places.rs",
    find: "            && !adt.adt_def.is_union()\n",
    replace: "            && adt.adt_def.is_struct()\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_generic_variant_field.rs$"],
    snapshots: true,
  },
  {
    name: "bound-place-read-any-value",
    breaks: "a binding that's a generic variant's `RefCell<T>` field's own place is a `RefCell<T>` of any value, which boxes every `RefCell` of an object, `{ value: [4, 5] }` (#503)",
    file: "src/lower/drops/ref_cells/places.rs",
    find: "                ExprKind::VarRef { id } => places.get(&id).is_some_and(|&field| generic(field)),\n",
    replace: "                ExprKind::VarRef { id } => places.get(&id).is_some_and(|_| false),\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_generic_variant_field.rs$"],
    snapshots: true,
  },
  {
    name: "into-inner-lends-its-cell",
    breaks:
      "a field's cell moved into its own `into_inner` lends it, so a flattened state's is refused as lent (#687)",
    file: "src/lower/drops/ref_cells/places.rs",
    find: "                | Std::CellReplaceWith\n                | Std::CellGet\n",
    replace: "                | Std::CellReplaceWith\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_into_inner.rs$"],
  },
];
