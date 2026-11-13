// Mutations of src/lower/analysis/cell_borrows/std_given.rs: what std's code
// runs of the crate's, by the bounds it's called under (ADR 0328), split
// from cell_borrows.rs by concept (#1334).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "std-impl-of-a-const-params-type-selected",
    breaks: "a `Buffer<SIZE>` in a `RefCell` an `Rc` holds asks std's impl of `[u8; SIZE]` where nothing says what `SIZE` is, a rustc ICE (#1017)",
    file: "src/lower/analysis/cell_borrows/std_given.rs",
    find: "    if ty.walk().any(|part| {\n        part.as_const()\n",
    replace: "    if false && ty.walk().any(|part| {\n        part.as_const()\n",
    tests: ["test/corpus.test.ts", "-t", "^const_param_in_a_ref_cell.rs$"],
  },
  {
    name: "closure-signature-walked",
    breaks:
      "a closure std's code is given is taken to run what its signature names, a function pointer, anything, so `types.borrow().iter().any(..)` is counted, `$borrow`",
    file: "src/lower/analysis/cell_borrows/std_given.rs",
    find: "            ty::Closure(closure, closure_args) => {\n                walk.skip_current_subtree();\n",
    replace: "            ty::Closure(closure, closure_args) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_quiet_borrows.rs$"],
    snapshots: true,
  },
  {
    name: "std-bound-impls-unrun",
    breaks:
      "std's code bounding a type of the crate's by a trait runs none of its impls, so a borrow held while `Vec<Peek>::clone` runs `Peek::clone`, which asks, isn't counted",
    file: "src/lower/analysis/cell_borrows/std_given.rs",
    find: "                    given.bodies.extend(\n                        (tcx.associated_item_def_ids(imp).iter())\n",
    replace:
      "                    given.bodies.extend(\n                        (tcx.associated_item_def_ids(imp).iter()).filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_borrow_in_std_clone.rs$"],
  },
  {
    name: "derive-fields-unread",
    breaks:
      "a derive std's code runs reads none of its fields, a `RefCell`'s clone of which borrows it",
    file: "src/lower/analysis/cell_borrows/std_given.rs",
    find: "                            for &t in &traits {\n                                bound_runs(recognition, impls, (field_ty, t), seen, given);\n",
    replace:
      "                            for &t in traits.iter().filter(|_| false) {\n                                bound_runs(recognition, impls, (field_ty, t), seen, given);\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_borrow_in_std_clone.rs$"],
  },
  {
    name: "param-bound-never-std",
    breaks:
      "a type parameter bound by `Hasher` runs anything, so a `HashSet` of a type whose `Hash` is `ptr::hash` is counted, `$borrowMut(t._updatedFibers, true)`",
    file: "src/lower/analysis/cell_borrows/std_given.rs",
    find: "                        .all(|imp| !imp.is_local() && recognition.in_sysroot(imp))\n",
    replace: "                        .all(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_identity_set.rs$"],
    snapshots: true,
  },
  {
    name: "cell-own-impl-borrows-nothing",
    breaks:
      "a `RefCell`'s own `PartialEq::eq` is taken to borrow nothing, so `a == b` while a `let` holds `a.borrow_mut()` doesn't panic (ADR 0863)",
    file: "src/lower/analysis/cell_borrows/std_given.rs",
    find: "        if recognition.is_borrowed_cell(cell) {\n            given.cells.push(Borrow {\n",
    replace:
      "        if false && recognition.is_borrowed_cell(cell) {\n            given.cells.push(Borrow {\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_eq_while_mut.rs$"],
  },
  {
    name: "std-impls-walk-every-part",
    breaks:
      "std's `Clone` of an `Option<Rc<RefCell<Vec<Task>>>>` runs each part's, a `fn`'s, so the queue is a counted cell (ADR 0868)",
    file: "src/lower/analysis/cell_borrows/std_given.rs",
    find: "        if let Some(bounds) = std_impl_bounds(recognition, part, &traits) {\n",
    replace: "        if let Some(bounds) = None::<Vec<(Ty<'tcx>, DefId)>> {\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_clone_through_rc.rs$"],
    snapshots: true,
  },
  {
    name: "std-impl-bounds-skipped",
    breaks:
      "what std's impl is bounded by runs nothing, so `Some(Asks).clone()`, whose `T: Clone` borrows `WHO`, doesn't panic `already borrowed` (ADR 0868)",
    file: "src/lower/analysis/cell_borrows/std_given.rs",
    find: "            for bound in bounds {\n                bound_runs(recognition, impls, bound, seen, given);\n            }\n",
    replace: "            let _ = bounds;\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_clone_through_rc.rs$"],
  },
  {
    name: "derive-walk-skipped",
    breaks:
      "a derived `clone` runs nothing, so `Holds`'s, whose field's `Clone` borrows `WHO`, doesn't panic `already borrowed` (ADR 0868)",
    file: "src/lower/analysis/cell_borrows/std_given.rs",
    find: "        bound_runs(recognition, impls, (ty, trait_id), &mut seen, &mut given);\n        return given;\n",
    replace: "        let _ = (ty, trait_id);\n        return given;\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_clone_through_rc.rs$"],
  },
  {
    name: "fn-pointer-clone-runs-it",
    breaks:
      "a function pointer's `Clone` may run it, so a cell of `Task(fn(..))`s borrowed while one's cloned is counted (ADR 0868)",
    file: "src/lower/analysis/cell_borrows/std_given.rs",
    find: "        return (!calls).then(Vec::new);\n",
    replace: "        return calls.then(Vec::new);\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_clone_through_rc.rs$"],
    snapshots: true,
  },
  {
    name: "trait-alias-selected",
    breaks:
      "a trait alias, `ptr::Thin`, is selected as a trait, and rustc's metadata has no items of it: an internal compiler error",
    file: "src/lower/analysis/cell_borrows/std_given.rs",
    find: "    if traits.iter().any(|&t| tcx.def_kind(t) != DefKind::Trait) {\n        return None;\n    }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
  },
  {
    name: "trait-of-arguments-selected",
    breaks:
      "a trait of more arguments, `FnMut(u32)`, is selected of its `Self` alone: an internal compiler error",
    file: "src/lower/analysis/cell_borrows/std_given.rs",
    find: "        if tcx.generics_of(t).count() != 1 {\n            return None;\n        }\n",
    replace: "",
    tests: ["test/lowering-cells.test.ts", "-t", "written by replace and take"],
  },
  {
    name: "library-identity-runs-code",
    breaks:
      "a library's type equal only to itself is taken to run unseen code by its `Hash` and `==`, so a borrow held while a `Set` adds one is checked, `$borrowMut`",
    file: "src/lower/analysis/cell_borrows/std_given.rs",
    find: "                if traits.iter().any(|&t| implemented(t) && !(identity && by_address(t))) {\n",
    replace: "                if traits.iter().any(|&t| implemented(t)) {\n",
    tests: [
      "test/crates.test.ts",
      "-t",
      "equal only to itself keys its consumer",
    ],
  },
  {
    name: "library-impl-always-there",
    breaks:
      "a library's type is taken to implement every trait by an impl of its own, `Borrow` too, which only std's blanket impl gives it, so a borrow of a `RefCell` of a set of it held while `contains` runs is refused as running the library's code",
    file: "src/lower/analysis/cell_borrows/std_given.rs",
    find: "                    impls.non_blanket_impls().contains_key(&own)\n",
    replace:
      "                    true || impls.non_blanket_impls().contains_key(&own)\n",
    tests: [
      "test/crates.test.ts",
      "-t",
      "equal only to itself keys its consumer",
    ],
  },
  {
    name: "std-function-given-its-types",
    breaks:
      "std's function given to std's, `get_or_insert_with(Vec::new)`, runs what each type it's of may, a `Vec<fn()>`'s `fn()`, so the thread-local is a checked cell, `$withBorrowMut(events, ..)` (ADR 0868, amended)",
    file: "src/lower/analysis/cell_borrows/std_given.rs",
    find: "            ty::FnDef(f, args) if recognition.is_std(f) => {\n",
    replace:
      "            ty::FnDef(f, args) if false && recognition.is_std(f) => {\n",
    tests: [
      "test/lowering-cell-borrows.test.ts",
      "-t",
      "std's function given to std's",
    ],
  },
  {
    name: "std-impl-bounds-of-a-param-type-unread",
    breaks: "std's impl of a type of a type parameter, `Rc<Node<K>>`'s `Clone`, is taken to run anything, so every momentary borrow of a generic graph's field is counted where code rust-js can't see runs (#499)",
    file: "src/lower/analysis/cell_borrows/std_given.rs",
    find: "    if ours || recognition.is_borrowed_cell(ty) || ty.has_aliases() {\n",
    replace: "    if ours || recognition.is_borrowed_cell(ty) || ty.has_param() || ty.has_aliases() {\n",
    tests: ["test/lowering-cell-borrows.test.ts", "-t", "type of a type parameter"],
  },
];
