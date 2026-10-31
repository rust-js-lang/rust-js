// Mutations of src/lower/drops/ref_cells.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "key-borrow-counts-its-type",
    breaks:
      "a thread-local's borrow held while its hooks run counts every cell of its type, so a controller's listeners, which no hook can borrow while it's held, are checked (ADR 0960, amended)",
    file: "src/lower/drops/ref_cells.rs",
    find: "                let own = (found.key).is_some_and(|key| self.krate.cell_borrows.reached_only_by_key(key));\n",
    replace: "                let own = false;\n",
    tests: [
      "test/lowering-cell-borrows.test.ts",
      "-t",
      "checks no other cell of its type",
    ],
  },
  {
    name: "key-counted-unasked",
    breaks:
      "a thread-local's own cell, counted only by its key's borrow held while its hooks run, is quiet where its key writes it, so a hook's `HOOKS.set(..)` doesn't panic as Rust's does (ADR 0960, amended)",
    file: "src/lower/drops/ref_cells.rs",
    find: "            || (of_key && key_counted())\n",
    replace: "",
    tests: [
      "test/lowering-cell-borrows.test.ts",
      "-t",
      "checks no other cell of its type",
    ],
  },
  {
    name: "untagged-boxed-ref-cell-unrefused",
    breaks:
      "an untagged enum's variant of a `RefCell<Vec<f64>>` the crate has as `{ value }` is tested `Array.isArray`, which it never is (ADR 0214, amended)",
    file: "src/lower/drops/ref_cells.rs",
    find: "            if self.is_std_type(held, StdItem::RefCell) && !found.is_value_cell(tcx, held) {\n",
    replace:
      "            if self.is_std_type(held, StdItem::RefCell) && !found.is_value_cell(tcx, held) && false {\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "variant of a RefCell the crate boxes",
    ],
  },
  {
    name: "plain-refcell-never",
    breaks:
      "a struct's `RefCell` field is `{ value }`, `root.times.value[..]` of the array JS made, where its declaration says it holds its value (ADR 0863, amended)",
    file: "src/lower/drops/ref_cells.rs",
    find: "            && struct_field(tcx, field.did)\n            && !self.is_value_cell(tcx, ty)\n",
    replace:
      "            && struct_field(tcx, field.did)\n            && false\n",
    tests: ["test/lowering-field-cells.test.ts"],
  },
  {
    name: "plain-refcell-any-borrow",
    breaks:
      'a field\'s `RefCell` borrowed, `fiber.r#return.borrow()`, is taken as lent, its every borrow counted on `$fieldCell(fiber, "return", true)`',
    file: "src/lower/drops/ref_cells.rs",
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
    name: "plain-refcell-lent-kept",
    breaks:
      "a field lent whole, `&x.f`, whose borrows the callee counts, is taken for one never checked",
    file: "src/lower/drops/ref_cells.rs",
    find: "                    uses.entry(field.did).or_insert(expr.span);\n",
    replace: "                    let _ = uses;\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_field_lent_held.rs$"],
  },
  {
    name: "plain-cells-asked-where-first",
    breaks:
      "which cells are plain is asked of each body where the first function that asks is: another's const parameter, `N`, isn't there, and rustc crashes",
    file: "src/lower/drops/ref_cells.rs",
    find: "                typing_env: ty::TypingEnv::post_analysis(tcx, owner),\n",
    replace: "                typing_env: self.typing_env,\n",
    tests: ["test/corpus.test.ts", "-t", "^const_param_drop_cells.rs$"],
  },
  {
    name: "replace-lends-the-cell",
    breaks:
      "a field's `RefCell` a `replace` writes is taken as lent, its borrows counted on its one cell, React's fiber's `return` `$fieldCell(fiber, \"return\", true)`",
    file: "src/lower/drops/ref_cells.rs",
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
    file: "src/lower/drops/ref_cells.rs",
    find: "                | Std::CellGetMut\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "a library's RefCells are the values"],
  },
  {
    name: "take-default-trusted",
    breaks:
      "a `take` that runs the crate's `Default` while it holds the cell is taken as holding it for a moment",
    file: "src/lower/drops/ref_cells.rs",
    find: "                    receiver.is_some_and(refcell)\n                        && !generic_args\n                            .types()\n                            .next()\n                            .is_some_and(|item| recognition.std_default(item))\n",
    replace: "                    receiver.is_some_and(refcell) && false\n",
    tests: ["test/lowering-cells.test.ts", "-t", "written by replace and take"],
  },
  {
    name: "library-plain-unread",
    breaks:
      "a library's field none of whose borrows it checks, `types`, is lent by its consumer unrefused (ADR 0863, amended)",
    file: "src/lower/drops/ref_cells.rs",
    find: "            None => (self.library_quiet).contains_key(&crate::lower::library::item_key(tcx, field)),\n",
    replace:
      "            None => false && (self.library_quiet).contains_key(&crate::lower::library::item_key(tcx, field)),\n",
    tests: ["test/crates.test.ts", "-t", "a library's RefCells are the values"],
  },
  {
    name: "value-cell-never",
    breaks:
      "a `RefCell` of an array is `{ value }`, `node.children.value`, where the guide says it's the array (ADR 0863)",
    file: "src/lower/drops/ref_cells.rs",
    find: "            && holds_itself(tcx, args.type_at(0))\n",
    replace: "            && false\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(rc_counts|refcell_quiet_borrows).rs$",
    ],
    snapshots: true,
  },
  {
    name: "boxed-ignores-generic",
    breaks:
      "generic code's `RefCell<T>` of any `T` is `{ value }` while the cells it's given are the objects they hold",
    file: "src/lower/drops/ref_cells.rs",
    find: "                    all |= !read && any_value(part);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^refcell_generic_mut_method.rs$"],
  },
  {
    name: "boxed-ignores-dyn",
    breaks:
      "an `Rc<RefCell<Circle>>` made an `Rc<RefCell<dyn Shape>>` is the object, its `dyn`'s pair `{ value, impl }` then another",
    file: "src/lower/drops/ref_cells.rs",
    find: "                    if shares {\n",
    replace: "                    if false && shares {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(dyn_ref_cells|rc_dyn_listeners).rs$",
    ],
  },
  {
    name: "boxed-library-unread",
    breaks:
      "a cell a library boxes is read by its consumer as the object (ADR 0863)",
    file: "src/lower/drops/ref_cells.rs",
    find: "        self.boxes_itself(tcx, ty) || self.library_has(tcx, ty, |f| &f.boxed).is_some()\n",
    replace: "        self.boxes_itself(tcx, ty)\n",
    tests: ["test/crates.test.ts", "-t", "a library's RefCells are the values"],
  },
  {
    name: "quiet-ignores-counted",
    breaks:
      "a `RefCell` a borrow of which is held while code that borrows it runs is never checked, and a second borrow runs on where Rust panics",
    file: "src/lower/drops/ref_cells.rs",
    find: "        if (self.counted.iter()).any(|&counted| crate::lower::analysis::may_be_cell(tcx, counted, ty))\n",
    replace:
      "        if (self.counted.iter()).any(|&counted| counted == ty && false)\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(refcell_double_borrow|refcell_borrow_in_std_closure|refcell_dyn_alias_borrow).rs$",
    ],
  },
  {
    name: "quiet-ignores-library-counted",
    breaks:
      "a cell a library counts is borrowed unchecked by its consumer (ADR 0863)",
    file: "src/lower/drops/ref_cells.rs",
    find: "            || self.library_has(tcx, ty, |f| &f.counted).is_some()\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "a library's RefCells are the values"],
  },
  {
    name: "quiet-ignores-impls",
    breaks:
      "a `RefCell` whose own `clone` runs a `Clone` that borrows it mutably is never checked, and the clone runs on where Rust panics",
    file: "src/lower/drops/ref_cells.rs",
    find: "        let asks = borrows.impls_ask(&recognition, ty, drops);\n",
    replace:
      "        let asks = false && borrows.impls_ask(&recognition, ty, drops);\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_clone_borrows_itself.rs$"],
  },
  {
    name: "unchecked-held-unrefused",
    breaks:
      "a consumer holds a borrow of a cell a library borrows unchecked while the library's code runs, which then can't find it borrowed (ADR 0863)",
    file: "src/lower/drops/ref_cells.rs",
    find: "        for &(ty, span) in unseen {\n",
    replace: "        for &(ty, span) in unseen.iter().filter(|_| false) {\n",
    tests: ["test/crates.test.ts", "-t", "a library's RefCells are the values"],
  },
  {
    name: "plain-lent-unrefused",
    breaks:
      "a consumer lends a library's field the library borrows unchecked, whose borrows then can't find the consumer's (ADR 0863, amended)",
    file: "src/lower/drops/ref_cells.rs",
    find: "            if !field.is_local()\n",
    replace: "            if false && !field.is_local()\n",
    tests: ["test/crates.test.ts", "-t", "a library's RefCells are the values"],
  },
  {
    name: "generic-box-unrefused",
    breaks:
      "a consumer boxes, of its generic code, a cell a library hands it as the object it holds (ADR 0863)",
    file: "src/lower/drops/ref_cells.rs",
    find: "                && self.library_has(tcx, ty, |f| &f.boxed).is_none()\n",
    replace: "                && false\n",
    tests: ["test/crates.test.ts", "-t", "a library's RefCells are the values"],
  },
  {
    name: "nullable-cell-field-unrefused",
    breaks:
      "a `#[rust_js::nullable]` `RefCell` field of a transparent struct, which is the cell, is taken for one that holds its value (ADRs 0584, 0863)",
    file: "src/lower/drops/ref_cells.rs",
    find: "                    if crate::lower::bindings::is_nullable(tcx, field)\n",
    replace:
      "                    if false && crate::lower::bindings::is_nullable(tcx, field)\n",
    tests: [
      "test/lowering-field-cells.test.ts",
      "-t",
      "transparent struct's nullable",
    ],
  },
  {
    name: "plain-refcell-bound-kept",
    breaks:
      "a struct's `RefCell` field a pattern binds, lent, is taken for one never checked",
    file: "src/lower/drops/ref_cells.rs",
    find: "                    self.found.entry(did).or_insert(field.pattern.span);\n",
    replace: "                    let _ = did;\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_bound_field_held.rs$"],
  },
  {
    name: "transparent-refcell-field-plain",
    breaks:
      "a transparent struct's `RefCell` of an array, React's taint `RequestCleanupQueue`, is taken for a field that holds its value, where the struct is the cell (ADRs 0584, 0863)",
    file: "src/lower/drops/ref_cells.rs",
    find: "        && crate::lower::recognition::transparent_field(tcx, tcx.adt_def(owner)).is_none()\n",
    replace: "",
    tests: [
      "test/crates.test.ts",
      "-t",
      "a library's RefCells are the values they hold",
    ],
  },
  {
    name: "transparent-refcell-holds-nothing",
    breaks:
      "a `RefCell` of a transparent struct of an array, `RefCell<Lines>`, is `{ value }`, not the array (ADRs 0584, 0863)",
    file: "src/lower/drops/ref_cells.rs",
    find: "        return holds_itself(tcx, tcx.type_of(field.did).instantiate_identity().skip_normalization());\n",
    replace: "        return false;\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_transparent.rs$"],
    snapshots: true,
  },
  {
    name: "transparent-refcell-of-a-number-object",
    breaks:
      "a `RefCell` of a transparent struct of a number, `RefCell<Meters>`, is taken for the object it holds, written in place (ADRs 0584, 0863)",
    file: "src/lower/drops/ref_cells.rs",
    find: "        return holds_itself(tcx, tcx.type_of(field.did).instantiate_identity().skip_normalization());\n",
    replace: "        return true;\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_transparent.rs$"],
  },
  {
    name: "field-refcell-plain-by-use",
    breaks:
      "no struct's `RefCell` field is one no borrow of which can meet another, each borrow of a fiber's `return` counted on `$fieldCell(fiber, \"return\", true)`",
    file: "src/lower/drops/ref_cells.rs",
    find: "                        found.quiet_fields.insert(field.did);\n",
    replace: "                        let _ = field;\n",
    tests: [
      "test/lowering-field-cells.test.ts",
      "-t",
      "whatever else has its type",
    ],
  },
  {
    name: "nullable-items-cell-refused",
    breaks:
      "a nullable field's `RefCell<Vec<Option<T>>>`, the array, is refused as a nullable `RefCell` that isn't plain",
    file: "src/lower/drops/ref_cells.rs",
    find: "                        && crate::lower::bindings::nullable_option(tcx, ty).is_some()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^nullable_items.rs$"],
  },
  {
    name: "derived-read-quiet",
    breaks:
      "a derive's read of a field, which borrows it while what it holds runs its own impl, is taken as one that can't meet another, where that impl may borrow it",
    file: "src/lower/drops/ref_cells.rs",
    find: "            if !quiet {\n                lent.entry(field).or_insert(span);\n            }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^refcell_field_derive_meets.rs$"],
  },
  {
    name: "generic-field-reads-box-all",
    breaks:
      "a generic struct's `RefCell<T>` field read boxes every `RefCell` of an object, so a `Root`'s `times` beside a `Cursor<T>` is a handle, `root.times[..]`, not the array a `let` holds (ADR 0863, amended)",
    file: "src/lower/drops/ref_cells.rs",
    find: "                    all |= !read && any_value(part);\n",
    replace: "                    all |= any_value(part);\n",
    tests: [
      "test/lowering-field-cells.test.ts",
      "-t",
      "whatever else has its type",
    ],
  },
  {
    name: "generic-field-objects-unboxed",
    breaks:
      "a loud generic struct's `RefCell<T>` field made of an array is the array where its concrete code reads it, which counts its borrows apart from its generic code's `$fieldCell`, so a borrow held there meets none",
    file: "src/lower/drops/ref_cells.rs",
    find: "                            boxed.insert(tcx.erase_and_anonymize_regions(made));\n",
    replace: "                            let _ = made;\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_generic_field_held.rs$"],
  },
  {
    name: "counted-field-quiet",
    breaks:
      "a field a borrow of which is held while code that may borrow it runs is taken as quiet, never checked",
    file: "src/lower/drops/ref_cells.rs",
    find: "                        counted_fields.entry(field).or_insert(found.span);\n",
    replace: "                        let _ = field;\n",
    tests: ["test/lowering-field-cells.test.ts", "-t", "lent is its one cell"],
  },
  {
    name: "library-quiet-fields-unlisted",
    breaks:
      "a library's contract says none of its fields is quiet, `fields` empty, so a consumer lends one unrefused (ADRs 0863, amended, 0906)",
    file: "src/lower/drops/ref_cells.rs",
    find: "        (self.quiet_fields.iter()).map(|&field| (field, crate::library::FieldFact::Quiet))\n",
    replace:
      "        (self.quiet_fields.iter()).filter(|_| false).map(|&field| (field, crate::library::FieldFact::Quiet))\n",
    tests: ["test/crates.test.ts", "-t", "a library's RefCells are the values"],
  },
  {
    name: "thread-local-storage-a-value",
    breaks:
      "a thread-local's storage, the closure its macro makes of its key, is taken for a value of its cell's type, so a crate whose generic code boxes every `RefCell` of an object is refused where a library shares that type as the object, React's `ReactFiberStack` (ADR 0863, amended)",
    file: "src/lower/drops/ref_cells.rs",
    find: "            if thread_local_key(tcx, tcx.typeck_root_def_id(body.def_id)) {\n",
    replace: "            if false {\n",
    tests: ["test/crates.test.ts", "-t", "two crates: thread_local_cell"],
  },
  {
    name: "thread-local-key-a-value",
    breaks:
      "a thread-local's key, `VALUE_STACK` of `VALUE_STACK.with_borrow_mut(..)`, is taken for a value of its cell's type, and the crate refused as its storage was (ADR 0863, amended)",
    file: "src/lower/drops/ref_cells.rs",
    find: "            walk.skip_current_subtree();\n            continue;\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: thread_local_cell"],
  },
  {
    name: "thread-local-init-a-value",
    breaks:
      "what a thread-local's `init` gives, its cell, is taken for a value of its type, `RefCell::new(vec![..])`, and the crate refused (ADR 0863, amended)",
    file: "src/lower/drops/ref_cells.rs",
    find: "        made.push(body.expr);\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: thread_local_cell"],
  },
  {
    name: "field-made-of-any-value",
    breaks:
      "a generic struct's `RefCell<T>` field made where the struct is, `Cursor { current: RefCell::new(value) }`, is a `RefCell<T>` of any value, which boxes every `RefCell` of an object, `{ value: [4, 5] }` (ADR 0863, amended)",
    file: "src/lower/drops/ref_cells.rs",
    find: "                    made.push(field.expr);\n",
    replace: "                    let _ = field;\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_generic_field_made.rs$"],
    snapshots: true,
  },
  {
    name: "quiet-library-key-counted",
    breaks:
      "a consumer's `with_borrow` of a library's thread-local `RefCell` none of whose borrows can meet another is counted, and refused as held while code rust-js can't see runs, where upstream reads `const child = current;` (ADR 0863, amended, gap rd76)",
    file: "src/lower/drops/ref_cells.rs",
    find: "                            || self.krate.quiet_library_keys.contains(&key))",
    replace: "                            || false)",
    tests: [
      "test/crates.test.ts",
      "-t",
      "two crates: thread_local_ref_cell_read",
    ],
  },
];
