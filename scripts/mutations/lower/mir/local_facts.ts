// Mutations of src/lower/mir/local_facts.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "quiet-with-borrow-of-another-module",
    breaks:
      "another module's quiet thread-local's `with_borrow` is given its value where it's lowered, not as that module's import reads it (ADR 0866, amended)",
    file: "src/lower/mir/local_facts.rs",
    find: "                        && tcx.parent_module_from_def_id(key) == self.module =>\n",
    replace: "                        =>\n",
    tests: ["test/lowering-module-variables.test.ts"],
  },
  {
    name: "facts-held-arms-unjoined",
    breaks:
      "a guard that holds still tests in each alternative of its arm, its graph made without the arms it goes on to (ADR 0790)",
    file: "src/lower/mir/local_facts.rs",
    find: "            graph: cfg::Graph::of(body, &held_arms, &payload_tests),\n",
    replace:
      "            graph: cfg::Graph::of(body, &Default::default(), &payload_tests),\n",
    tests: [
      "test/lowering-labels.test.ts",
      "-t",
      "a block the program labeled that ends a case",
    ],
  },
  {
    name: "facts-patterned-none",
    breaks:
      "a temporary a `let` pattern of one field takes apart is folded, `const getProps = f().get_props`",
    file: "src/lower/mir/local_facts.rs",
    find: "            patterned: self.mir_patterned(body),\n",
    replace: "            patterned: Default::default(),\n",
    tests: ["test/lowering-matches.test.ts", "-t", "pattern of one field"],
  },
  {
    name: "facts-jsx-props-none",
    breaks:
      "a component's props with a flattened field are flattened before JSX takes them apart",
    file: "src/lower/mir/local_facts.rs",
    find: "            jsx_props: self.mir_jsx_props(body),\n",
    replace: "            jsx_props: Default::default(),\n",
    tests: ["test/jsx-props.test.ts"],
  },
  {
    name: "quiet-field-calls-unseen",
    breaks:
      "no call of a struct's quiet field is found, so each is checked by its type's borrows elsewhere (ADR 0863, amended)",
    file: "src/lower/mir/local_facts.rs",
    find: "                    quiet.calls.insert(block);\n                    if matches!(known, Std::Borrow { .. } | Std::TryBorrow { .. })",
    replace: "                    if matches!(known, Std::Borrow { .. } | Std::TryBorrow { .. })",
    tests: [
      "test/lowering-field-cells.test.ts",
      "-t",
      "whatever else has its type",
    ],
  },
  {
    name: "module-variable-set-unseen",
    breaks:
      "a thread-local that's its module's variable is set as a cell, `$borrowMut(current)` (ADR 0270, amended)",
    file: "src/lower/mir/local_facts.rs",
    find: "                        quiet.calls.insert(block);\n                    }\n                    if crate::lower::bindings::nullable_local(tcx, key) {\n",
    replace:
      "                    }\n                    if crate::lower::bindings::nullable_local(tcx, key) {\n",
    tests: [
      "test/lowering-field-cells.test.ts",
      "-t",
      "whatever else has its type",
    ],
  },
  {
    name: "quiet-guards-unmoved",
    breaks:
      "a quiet field's guard moved to a variable, `let times = root.times.borrow(); .. drop(times);`, releases what it never counted, `$unborrow(times)` (ADR 0863, amended)",
    file: "src/lower/mir/local_facts.rs",
    find: "            quiet.guards.extend(more);\n",
    replace: "            let _ = more;\n            break;\n",
    tests: [
      "test/lowering-field-cells.test.ts",
      "-t",
      "whatever else has its type",
    ],
  },
  {
    name: "nullable-local-calls-unseen",
    breaks:
      "no call that writes a nullable thread-local is found, so what may be `undefined` is stored as it is, `NEXT = fiber.stateNode`, where Rust's `None` is `null` (ADR 0586, amended)",
    file: "src/lower/mir/local_facts.rs",
    find: "                        quiet.nullable.insert(block);\n",
    replace: "",
    tests: [
      "test/lowering-nullable-reads.test.ts",
      "-t",
      "a place a test shows present is stored as it is",
    ],
  },
  {
    name: "nullable-local-get-unmarked",
    breaks:
      "a nullable thread-local's `get` isn't known `null` or a value, `return CURRENT ?? null;` (ADR 0949)",
    file: "src/lower/mir/local_facts.rs",
    find: "                Some(Std::CellGet)\n",
    replace: "                Some(Std::Drop)\n",
    tests: ["test/lowering-nullable-reads.test.ts"],
  },
  {
    name: "nullable-local-clone-unread",
    breaks:
      "a nullable thread-local's `with_borrow(Clone::clone)` may be `undefined`, so popSearchTarget returns `searchTarget ?? null`",
    file: "src/lower/mir/local_facts.rs",
    find: "                    .is_some_and(|t| tcx.is_lang_item(t, LangItem::Clone))\n            {\n                quiet.nullable.insert(block);\n",
    replace:
      "                    .is_some_and(|t| tcx.is_lang_item(t, LangItem::Clone))\n            {\n",
    tests: [
      "test/lowering-nullable-reads.test.ts",
      "-t",
      "a nullable parameter, a nullable thread-local",
    ],
  },
  {
    name: "whole-variable-unknown",
    breaks:
      "a variable given a WebIDL integer, `let status = response.status()`, isn't known whole, `${$displayF64(status)}` (ADR 0994, amended)",
    file: "src/lower/mir/local_facts.rs",
    find: "            read.facts = read.facts.with(js::Known::Whole);\n",
    replace: "",
    tests: ["test/webapi-bindings.test.ts", "-t", "WebIDL integer"],
  },
  {
    name: "made-pointer-cast-unfollowed",
    breaks:
      "a `let f: Rc<dyn Fn()> = Rc::new(..)`, whose unsizing cast MIR makes, isn't the program's own `Rc`, so its closure is a function declared, not a `const` of its arrow (ADR 0882, amended)",
    file: "src/lower/mir/local_facts.rs",
    find: "                | Rvalue::Cast(_, Operand::Copy(read) | Operand::Move(read), _)\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(dyn_fn_generic_closure_drops|rc_fn_ptr_eq|self_handle).rs$",
    ],
    snapshots: true,
  },
  {
    name: "tried-int-never",
    breaks:
      'an integer\'s `try_from` only tested is `$tryFromInt(..)` and `value.TAG !== "Ok"`, not its range test, `if (stairsNum < 0)` (#366)',
    file: "src/lower/mir/local_facts.rs",
    find: "        self.tried_ints.contains(&block)\n",
    replace: "        false && self.tried_ints.contains(&block)\n",
    tests: ["test/corpus.test.ts", "-t", "^try_from_int_tested.rs$"],
    snapshots: true,
  },
  {
    name: "momentary-borrow-held",
    breaks:
      "a borrow of a counted `RefCell` held only while nothing can ask, `queue.borrow_mut().push(x)`, is counted and released in a `finally`, `const b = $borrowMut(queue, true); try { b.push(x); } finally { $unborrow(b); }`, not checked alone, `$borrowMut(queue).push(x)` (rd218)",
    file: "src/lower/mir/local_facts.rs",
    find: "                    quiet.momentary.insert(block);\n                    quiet.guards.insert(destination.local);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^refcell_vec_bounds.rs$"],
    snapshots: true,
  },
  {
    name: "lasting-of-any-call",
    breaks:
      "a reference any call gives is lasting, so a field a borrow of a local gives, `(outer.inner().f)({ outer.inner.f = triple; n })`, is read after what's given (rd285)",
    file: "src/lower/mir/local_facts.rs",
    find: "                    && matches!(output.kind(), ty::Ref(region, ..) if region.is_static())\n",
    replace: "                    && matches!(output.kind(), ty::Ref(..))\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^field_functions_called_after_arguments.rs$",
    ],
  },
  {
    name: "lasting-parameter-rebound",
    breaks:
      "a `&T` parameter the body assigns again is lasting, so `(hooks.f)({ hooks = other; n })` calls `other`'s (rd285)",
    file: "src/lower/mir/local_facts.rs",
    find: "            .filter(|&local| shared(body.local_decls[local].ty) && written(local) == 0);\n",
    replace:
      "            .filter(|&local| shared(body.local_decls[local].ty));\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^field_functions_called_after_arguments.rs$",
    ],
  },
  {
    name: "found-index-none-null",
    breaks:
      "a slice's `position` searched by `indexOf` is tested `index != null`, its `-1` taken for `Some`, so `children.splice(-1, 1)` removes the last",
    file: "src/lower/mir/local_facts.rs",
    find: "        self.offsets.in_units(local) || self.found_indexes.holds(local)\n",
    replace: "        self.offsets.in_units(local)\n",
    tests: ["test/corpus.test.ts", "-t", "^position_index_of.rs$"],
  },
  {
    name: "held-reference-a-place",
    breaks:
      "a `&mut` moved out of a `Result`'s payload, `?` of `get_mut(k).ok_or_else(..)`, is taken as its place, and `*r = v` is `result._0 = v`, the map's entry left as it was (#466)",
    file: "src/lower/mir/local_facts.rs",
    find: "                        None => Made::Other,\n",
    replace: "                        None => Made::Place,\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_held_written_whole.rs$"],
  },
  {
    name: "reborrow-of-held-a-place",
    breaks:
      "`&mut *h.account`, a reborrow of a reference a field holds, is taken as its place, and `*r = v` is `h.account = v` (#466)",
    file: "src/lower/mir/local_facts.rs",
    find: "                    Some(PlaceElem::Deref) | None => Made::Other,\n",
    replace: "                    Some(PlaceElem::Deref) | None => Made::Place,\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_held_written_whole.rs$"],
  },
  {
    name: "moved-reference-a-place",
    breaks:
      "a move of any `&mut`, `_8 = move _7` of what `?` read from a payload, is taken as its place, and `*r = v` replaces the payload (#466)",
    file: "src/lower/mir/local_facts.rs",
    find: "                        Made::Of(of) => places.contains(of),\n",
    replace: "                        Made::Of(_) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_held_written_whole.rs$"],
  },
  {
    name: "index-mut-not-a-place",
    breaks:
      "`v[i] = p` of a `Vec` of objects is `$assign(v[i], p)`, the old object made `p`, as `index_mut`'s `&mut` isn't taken as the item's place (#349)",
    file: "src/lower/mir/local_facts.rs",
    find: "                        self.tcx.is_lang_item(t, LangItem::IndexMut) || self.tcx.is_lang_item(t, LangItem::DerefMut)\n",
    replace: "                        self.tcx.is_lang_item(t, LangItem::DerefMut)\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_item_written_whole.rs$"],
    snapshots: true,
  },
  {
    name: "deref-mut-not-a-place",
    breaks:
      "`*cell.borrow_mut() = v` of a `RefCell` of a `dyn` is `$assign($borrowMut(cell).value, v)`, as a guard's `deref_mut` isn't taken as its value's place",
    file: "src/lower/mir/local_facts.rs",
    find: "                        self.tcx.is_lang_item(t, LangItem::IndexMut) || self.tcx.is_lang_item(t, LangItem::DerefMut)\n",
    replace: "                        self.tcx.is_lang_item(t, LangItem::IndexMut)\n",
    tests: ["test/corpus.test.ts", "-t", "^(dyn_ref_cells|lock_destructors).rs$"],
    snapshots: true,
  },
  {
    name: "nullable-lend-unmarked",
    breaks:
      "no `with` or `with_borrow_mut` lends a nullable thread-local, so what's written through the lend of its module's variable is stored as it is, `current = noneOf(1)` (#541)",
    file: "src/lower/mir/local_facts.rs",
    find: "                quiet.nullable_lent.insert(block);\n",
    replace: "",
    tests: ["test/lowering-nullable-borrows.test.ts", "-t", "written through a borrow"],
  },
  {
    name: "local-with-not-quiet",
    breaks:
      "a `with` of a module's variable isn't quiet, so a nullable one's cell is lent whole, kept a `$nullableCell`, not its module's variable (#541)",
    file: "src/lower/mir/local_facts.rs",
    find: "                Some(Std::LocalBorrow { .. } | Std::LocalWith)\n",
    replace: "                Some(Std::LocalBorrow { .. })\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_written_through_borrows.rs$"],
    snapshots: true,
  },
  {
    name: "alike-set-unsaid",
    breaks: "a nullable thread-local nothing reads apart isn't said so where it's set, so it stores what it's given `?? null` (#665)",
    file: "src/lower/mir/local_facts.rs",
    find: "                            quiet.alike.insert(block);\n",
    replace: "",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "a nullable thread-local nothing reads apart"],
  },
  {
    name: "swapped-flattened-unseen",
    breaks:
      "no `swap` of a flattened state is found, so it's `$refCellSwap`, whose `[a.value, b.value] = [b.value, a.value]` rebinds the handle's object (#687)",
    file: "src/lower/mir/local_facts.rs",
    find: "                    quiet.flattened.insert(block, swapped);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_cell_swapped.rs$"],
  },
  {
    name: "swapped-flattened-reborrow-unfollowed",
    breaks:
      "a flattened state given to a `swap` reborrowed, `&*r` of `r = &task.state`, isn't found, so `other.swap(&task.state)` is `$refCellSwap` of the handle (#687)",
    file: "src/lower/mir/local_facts.rs",
    find: "                    while let Some(Some(Rvalue::Ref(_, _, lent))) = assigned.get(&local)\n",
    replace: "                    while false\n                        && let Some(Some(Rvalue::Ref(_, _, lent))) = assigned.get(&local)\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_cell_swapped.rs$"],
  },
  {
    name: "tried-guard-of-a-quiet-field-released",
    breaks:
      "a quiet field's `try_borrow` guard, which counted nothing, is released where it's dropped, `$unborrow(state)` of a flattened state's handle (#687)",
    file: "src/lower/mir/local_facts.rs",
    find: "                    if matches!(known, Std::Borrow { .. } | Std::TryBorrow { .. }) && destination.projection.is_empty()\n",
    replace: "                    if matches!(known, Std::Borrow { .. }) && destination.projection.is_empty()\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_tried.rs$"],
    snapshots: true,
  },
];
