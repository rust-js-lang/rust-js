// Mutations of src/lower/analysis/cell_borrows.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "cell-of-a-run-of-no-args-kept-its-parameters",
    breaks:
      "a `RefCell<I::IntoIter>` the crate's `from_iter` borrows, run by std's `collect` of no args a type says, keeps `from_iter`'s `I`, which its caller's args are given in its place, a rustc panic (#1017)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "                    ty::Param(p) => Ty::new_fresh(tcx, p.index),\n",
    replace: "                    ty::Param(_) => t,\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_of_an_iterator_std_runs.rs$"],
  },
  {
    name: "cell-of-a-run-of-no-args-kept-its-const-parameters",
    breaks: "a `RefCell<[u32; N]>` a `Fixed<N>`'s `from_iter` borrows, run by std's `collect`, keeps its impl's `N`, a rustc panic (#1017)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "                    ty::ConstKind::Param(p) => ty::Const::new_fresh(tcx, p.index),\n",
    replace: "                    ty::ConstKind::Param(_) => c,\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_of_an_iterator_std_runs.rs$"],
  },
  {
    name: "cell-key-never-same",
    breaks:
      "a thread-local's borrow is never its own, so `LOG`'s second borrow in its own closure isn't counted and doesn't throw",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "        (Cell::Key(x), Cell::Key(y)) => x == y,\n",
    replace: "        (Cell::Key(_), Cell::Key(_)) => false,\n",
    tests: [
      "test/lowering-cells.test.ts",
      "-t",
      "closures call only what can't borrow",
    ],
  },
  {
    name: "crate-call-borrows-nothing",
    breaks:
      "a function of the crate's is taken to borrow nothing, so `logged()` in `LOG`'s closure makes `LOG` plain, and its borrow doesn't throw",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "    match id.is_local() && !is_binding(tcx, id) {\n        true => Called::Body(id, instance.args),\n",
    replace:
      "    match id.is_local() && !is_binding(tcx, id) {\n        true => Called::Nothing,\n",
    tests: [
      "test/lowering-cells.test.ts",
      "-t",
      "closures call only what can't borrow",
    ],
  },
  {
    name: "crate-call-borrows-nothing-inside",
    breaks:
      "a function of the crate's is taken to borrow nothing, so a thread-local's borrow inside its own closure isn't counted",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "    match id.is_local() && !is_binding(tcx, id) {\n        true => Called::Body(id, instance.args),\n",
    replace:
      "    match id.is_local() && !is_binding(tcx, id) {\n        true => Called::Nothing,\n",
    tests: ["test/corpus.test.ts", "-t", "thread_local_borrow_inside"],
  },
  {
    name: "crate-call-borrows-anything",
    breaks:
      "a function of the crate's may borrow anything, as before, so `timerQueue` and `taskQueue` stay `{ value }` cells",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "        Called::Body(body, args) => found.runs.push((body, Some(args))),\n",
    replace: "        Called::Body(..) => found.any = true,\n",
    tests: [
      "test/lowering-cells.test.ts",
      "-t",
      "closures call only what can't borrow",
    ],
  },
  {
    name: "body-asks-of-nothing",
    breaks:
      "a thread-local's `with_borrow_mut` closure that borrows it again keeps it plain, unchecked",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "            Some(Some(summary)) => summary.asks(tcx, &self.closed, held),\n",
    replace: "            Some(Some(_)) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "thread_local_borrow_inside"],
  },
  {
    name: "refcell-write-borrows-nothing",
    breaks:
      "a `RefCell`'s `replace` borrows nothing, so a guard held across it isn't counted and the `replace` doesn't throw where Rust panics",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "        Some(Std::CellReplace | Std::CellTake | Std::CellSwap | Std::CellReplaceWith) if refcell => Some(true),\n",
    replace: "",
    tests: ["test/lowering-cells.test.ts", "-t", "written by replace and take"],
  },
  {
    name: "std-call-runs-nothing",
    breaks:
      "std's code given a closure that borrows a cell held runs nothing, so the borrow isn't counted and the closure's second one runs on where Rust panics (ADR 0863)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "        false if recognition.is_std(id) => Called::Std(id, instance.args),\n",
    replace: "        false if recognition.is_std(id) => Called::Nothing,\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_borrow_in_std_closure.rs$"],
  },
  {
    name: "cell-may-be-ignores-dyn",
    breaks:
      "a `RefCell<dyn Shape>` held isn't the `RefCell<Circle>` it was made of, so a borrow of the concrete one runs on where Rust panics",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "        (ty::Dynamic(..), _) | (_, ty::Dynamic(..)) if unsized_here => true,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^refcell_dyn_alias_borrow.rs$"],
  },
  {
    name: "place-behind-ref-dropped",
    breaks:
      "a place behind a reference, `*state`, is taken as dropped where it's read, so a generic `Hash` runs anything and its cell is counted",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "        let behind_ref = matches!(e.kind, ExprKind::Deref { arg } if thir[arg].ty.is_ref());\n",
    replace: "        let behind_ref = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_identity_set.rs$"],
    snapshots: true,
  },
  {
    name: "impls-ask-nothing",
    breaks:
      "a `RefCell`'s own `clone`, which holds it while what it holds clones, is taken to ask nothing, so a `Clone` that borrows it mutably runs on where Rust panics",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "        given.any\n            || given.cells.iter().any(|&theirs| asks(tcx, &self.closed, theirs, held))\n            || (given.bodies.iter()).any(|&(body, args)| self.runs_asks(tcx, body, args, held, drops))\n    }\n\n    /// Whether calling",
    replace: "        false\n    }\n\n    /// Whether calling",
    tests: ["test/corpus.test.ts", "-t", "^refcell_clone_borrows_itself.rs$"],
  },
  {
    name: "impls-ask-of-what-it-hasnt",
    breaks:
      "a `RefCell<Vec<&dyn Fn()>>`'s `PartialEq` and `Debug`, which it hasn't, are taken to run what a `dyn Fn()` may, so each moment's borrow of a list of listeners is checked, `$borrowMut(listeners, true)`",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "        for trait_id in traits.into_iter().flatten().filter(|&t| has(t)) {\n",
    replace: "        for trait_id in traits.into_iter().flatten() {\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_listeners_momentary.rs$"],
    snapshots: true,
  },
  {
    name: "impls-had-of-a-parameter-unknown",
    breaks:
      "a `RefCell<T>` of a type parameter's has no impl of what's asked, as rustc can't select one, so a generic `Clone` that borrows its cell mutably runs on where Rust panics",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "            if cell.has_param() || cell.has_aliases() {\n                return true;\n            }\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^cell_of_an_iterator_std_runs.rs$|^const_param_in_a_ref_cell.rs$",
    ],
  },
  {
    name: "unseen-never",
    breaks:
      "a borrow held while a library's code runs is taken to run only what rust-js sees, so a consumer holds one of a cell the library borrows unchecked (ADR 0863)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "            Called::Any => self.anything_asks(tcx, held),\n        }\n    }\n\n    /// Whether a cell's own call",
    replace: "            Called::Any => false,\n        }\n    }\n\n    /// Whether a cell's own call",
    tests: ["test/crates.test.ts", "-t", "a library's RefCells are the values"],
  },
  {
    name: "derived-methods-run-anything",
    breaks:
      "a derived `clone` is a body never lowered, which may run anything, so a cell borrowed while a `Task` is cloned is counted (ADR 0868)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "    if id.is_local() && derived_method(tcx, id).is_some() {\n        return Called::Std(id, instance.args);\n    }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^refcell_clone_through_rc.rs$"],
    snapshots: true,
  },
  {
    name: "unargued-summary-instantiated",
    breaks:
      "a generic impl's method std's code runs, of no args a type says, has its own parameters read with its caller's: the compiler crashes instantiating them (#240)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "    if args.is_none()\n        && (summary.drops.iter().any(typed)\n",
    replace: "    if false\n        && (summary.drops.iter().any(typed)\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_iterator_std_runs.rs$"],
  },
  {
    name: "intrinsic-runs-anything",
    breaks:
      "an intrinsic, `vec![..]`'s `write_box_via_move`, may run anything, so a borrow held across `vec![..]` is counted, and every borrow of its cell's type",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "            false => Called::Std(id, instance.args),\n",
    replace: "            false => Called::Any,\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_vec_element_places.rs$"],
    snapshots: true,
  },
  {
    name: "fn-pointer-call-runs-anything",
    breaks:
      "a fn pointer's call may run anything, as before, so `clz32`'s, while `root.entanglements` is held, counts it",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "                None => match pointers.may_be(tcx, thir[fun].ty) {\n",
    replace:
      "                None => match None::<&[(DefId, ty::GenericArgsRef<'tcx>)]> {\n",
    tests: [
      "test/lowering-cell-borrows.test.ts",
      "-t",
      "fn pointer runs what the crate made",
    ],
  },
  {
    name: "own-clone-runs-anything",
    breaks:
      "a clone of a type parameter's whose `Clone` takes no dictionary is taken to run code rust-js can't see, so a library's unchecked borrow held across it is refused (ADR 0914, amended)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "    if tcx.is_lang_item(def_id, LangItem::CloneFn) && args.types().next().is_some_and(clones_itself) {\n",
    replace:
      "    if false && tcx.is_lang_item(def_id, LangItem::CloneFn) && args.types().next().is_some_and(clones_itself) {\n",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "judged by what its callers give",
    ],
  },
  {
    name: "lent-key-reached-only-by-key",
    breaks:
      "a thread-local whose `with` lends its cell is taken for one only its key reaches, so a `&RefCell` of its type borrowed mutably while its hooks run, `clear(hooks)`, isn't checked, where Rust panics (ADR 0960, amended)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "        self.closed.contains(&key)\n",
    replace: "        true\n",
    tests: [
      "test/lowering-cell-borrows.test.ts",
      "-t",
      "checks no other cell of its type",
    ],
  },
  {
    name: "cell-dyn-anywhere",
    breaks:
      "a `dyn` in a `Vec`'s items is taken to be any type, so a `RefCell<Vec<Rc<Node>>>` may be React's commit hooks' `RefCell<Vec<Rc<dyn Fn()>>>`, borrowed while they run, and each borrow of it is checked (#374, ADR 0960)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "        (ty::Dynamic(..), _) | (_, ty::Dynamic(..)) if unsized_here => true,\n",
    replace: "        (ty::Dynamic(..), _) | (_, ty::Dynamic(..)) => true,\n",
    tests: [
      "test/lowering-cell-borrows.test.ts",
      "-t",
      "field.s hooks held while they run meet no cell",
    ],
  },
  {
    name: "cell-unsized-tail-unseen",
    breaks:
      "a `RefCell<T>`'s `T` isn't where a value is made unsized in place, so a `RefCell<dyn Shape>` isn't the `RefCell<Circle>` it was made of, and a borrow of the concrete one runs on where Rust panics (ADR 0960)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "            ty::Param(param) => return Some(param.index as usize),\n",
    replace: "            ty::Param(_) => return None,\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_dyn_alias_borrow.rs$"],
  },
  {
    name: "cell-borrow-runs-unseen",
    breaks:
      "a cell's borrow is code rust-js can't see, so a consumer's borrow of a library's `RefCell<Vec<String>>` held while it borrows another is refused (ADR 0863, amended)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "            Some(crate::lower::Std::Borrow { .. } | crate::lower::Std::TryBorrow { .. })\n        ) {\n            return false;\n",
    replace:
      "            Some(crate::lower::Std::Borrow { .. } | crate::lower::Std::TryBorrow { .. })\n        ) && false {\n            return false;\n",
    tests: ["test/crates.test.ts", "-t", "a library's RefCells are the values"],
  },
  {
    name: "unseen-runs-what-loads",
    breaks:
      "what JS runs is taken to borrow what a module runs as it loads, `js::on_load!`'s own set, so React's DEV dispatchers stay `{ value: null }` cells checked around `setH` (ADR 0580, amended)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "            .filter(|&(&id, _)| !crate::lower::bindings::is_on_load(tcx, id))\n",
    replace: "",
    tests: [
      "test/lowering-cell-borrows.test.ts",
      "-t",
      "a thread-local set only where its module loads",
    ],
  },
  {
    name: "unseen-never-reaches-public",
    breaks:
      "a `pub` thread-local, which another crate may borrow, is taken to be borrowed by the crate's code alone, its borrow across JS unchecked (ADR 0580, amended)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "    keys.retain(|&key| !open.contains(&key) && !tcx.visibility(key).is_public());\n",
    replace: "    keys.retain(|&key| !open.contains(&key));\n",
    tests: [
      "test/lowering-cell-borrows.test.ts",
      "-t",
      "a thread-local set only where its module loads",
    ],
  },
  {
    name: "unseen-asks-everything",
    breaks:
      "what JS runs is taken to borrow any cell, so a thread-local set only where its module loads is a checked `{ value }` cell (ADR 0580, amended)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "            Cell::Key(key) if self.closed.contains(&key) => {\n",
    replace:
      "            Cell::Key(key) if false && self.closed.contains(&key) => {\n",
    tests: [
      "test/lowering-cell-borrows.test.ts",
      "-t",
      "a thread-local set only where its module loads",
    ],
  },
  {
    name: "own-key-cell-a-fields",
    breaks:
      'a thread-local only its key reaches, set while a field of its cell\'s type is borrowed, may be that field, so every such field is a checked cell, `$borrow($fieldCell(fiber, "parent"), true)` (ADR 0960, amended)',
    file: "src/lower/analysis/cell_borrows.rs",
    find: "            !closed.contains(&key) && held(key).is_none_or(|cell| alike(cell, ty))\n",
    replace: "            held(key).is_none_or(|cell| alike(cell, ty))\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^thread_local_set_of_a_field_borrow.rs$",
    ],
    snapshots: true,
  },
  {
    name: "class-trait-call-runs-anything",
    breaks:
      "a default's call of a trait of classes' method, of a `Self` not known, may run anything, so its borrow of the field another method gives, `self.internal_root().borrow()`, is counted, `$borrow($fieldCell(this, ..))` (rd304)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "            return Called::Bodies(may_be);\n",
    replace: "            return Called::Any;\n",
    tests: ["test/classes-traits.test.ts", "-t", "prototype's"],
  },
  {
    name: "class-trait-call-asks-nothing",
    breaks:
      "a default's call of a trait of classes' method, of a `Self` not known, asks nothing of a borrow held across it, though one class's borrows its cell, which then doesn't panic as std's does (rd304)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "                (bodies.into_iter()).any(|(body, args)| self.runs_asks(tcx, body, Some(args), held, drops))\n",
    replace: "                bodies.is_empty()\n",
    tests: ["test/corpus.test.ts", "-t", "^class_trait_field_held.rs$"],
  },
  {
    name: "seen-borrows-own-only",
    breaks:
      "what a body borrows by code rust-js sees leaves out the crate's bodies it runs, so `t.borrow_mut().push(size(t))` of a `size` that borrows `t` meets nothing",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "                next.extend(theirs.iter().map(|&borrow| of_args(tcx, borrow, args)));\n",
    replace: "",
    tests: ["test/lowering-cell-borrows.test.ts", "-t", "only a call's receiver"],
  },
  {
    name: "seen-asks-nothing",
    breaks:
      "a call's argument that borrows the receiver guard's cell by a body of the crate's is taken as meeting nothing",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "            Called::Body(body, args) => self.seen_asks(tcx, body, Some(args), held),\n",
    replace: "            Called::Body(..) => false,\n",
    tests: ["test/lowering-cell-borrows.test.ts", "-t", "only a call's receiver"],
  },
  {
    name: "classes-always-meet",
    breaks:
      "a borrow of a cell of one class by its flow is taken as meeting one of another, so `drain_into`'s walk of one queue's items while another's are pushed to is counted, `$borrow(from.items, true)`, where nothing can meet it (rd482)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "        (Some(CellClass::Own(a)), Some(CellClass::Own(b))) => a == b,\n",
    replace: "        (Some(CellClass::Own(_)), Some(CellClass::Own(_))) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^held_borrow_of_own_cell.rs$"],
    snapshots: true,
  },
  {
    name: "own-class-unseen-asks-nothing",
    breaks:
      "code rust-js can't see is taken as borrowing no cell only the crate's code reaches, so a thread-local's queue walked while a callback runs that borrows it by its key isn't counted, and its `try_borrow_mut` finds it free where Rust's finds it borrowed (rd482)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "                Some(unseen) => unseen.iter().any(|&theirs| asks(tcx, &self.closed, theirs, held)),\n",
    replace: "                Some(_) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^held_borrow_of_own_cell.rs$"],
  },
  {
    name: "borrows-unclassed",
    breaks:
      "a body's borrows of a cell aren't classed by its flow, so a call that borrows only another queue's items is taken as meeting the walk of one's, counted (rd482)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "                            class: flow.class_of(thir, body, at),\n",
    replace: "                            class: None,\n",
    tests: ["test/corpus.test.ts", "-t", "^held_borrow_of_own_cell.rs$"],
    snapshots: true,
  },
  {
    name: "held-borrow-unclassed",
    breaks:
      "a held borrow isn't classed by its flow, so a walk of one queue's items while another's are pushed to is taken as meeting them, counted (rd482)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "                class: self.flow.class_of(thir, body, call),\n",
    replace: "                class: self.flow.class_of(thir, body, call).filter(|_| false),\n",
    tests: ["test/corpus.test.ts", "-t", "^held_borrow_of_own_cell.rs$"],
    snapshots: true,
  },
  {
    name: "bounds-ask-of-anything-nothing",
    breaks:
      "a function a `replace_with` runs that may borrow anything, which no summary says, is taken to ask nothing of a borrow held while it runs, so a guard of a cell it borrows is quiet and `try_borrow_mut` sees no borrow (#687)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "                _ => true,\n            })\n    }\n",
    replace: "                _ => false,\n            })\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_replaced_with.rs$"],
  },
  {
    name: "cell-call-runs-nothing-of-its-bounds",
    breaks:
      "a cell's own call, `replace_with`, runs nothing but its borrow, so a guard of another cell held across it whose function borrows that cell is quiet, and the JS writes the cell where Rust's `borrow_mut` panics, or its `try_borrow_mut` sees no borrow (#687)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "        asks(recognition.tcx, &self.closed, theirs, held) || self.bounds_ask(recognition, called, held, drops)\n",
    replace: "        asks(recognition.tcx, &self.closed, theirs, held)\n",
    tests: ["test/corpus.test.ts", "-t", "^(flattened_state_replaced_with|replace_with_held_across).rs$"],
  },
  {
    name: "std-call-drops-unasked",
    breaks: "what a std call drops through a reference, `g.borrow_mut().clear()`'s items, is taken to drop nothing, so a cell their destructors borrow is quiet: `try_borrow_mut` folded to `true`, `already borrowed` never panics (#1661)",
    file: "src/lower/analysis/cell_borrows.rs",
    find: "            for ty in super::may_run::std_call_drops(tcx, id, args) {\n",
    replace: "            for ty in super::may_run::std_call_drops(tcx, id, args).into_iter().take(0) {\n",
    tests: ["test/corpus.test.ts", "-t", "^ref_cell_borrowed_(by_a_clear_through_a_reference|by_a_truncate_retain_or_map_clear|by_a_thread_locals_clear|mutably_by_a_clear_through_a_reference).rs$"],
  },
];
