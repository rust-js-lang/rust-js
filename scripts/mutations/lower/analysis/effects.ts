// Mutations of src/lower/analysis/effects.rs, what a statement may write,
// the query the MIR intervals' facts are kept by (#2112 B). Each makes the
// query keep a fact a write reaches, which its shadow's row sees.
import type { Mutation } from "../../../mutations";

const file = "src/lower/analysis/effects.rs";

export const mutations: Mutation[] = [
  {
    name: "effects-call-result-shallow",
    breaks:
      "a call's result holds only the pointers it's given, not what they reach, so `deref_mut(&mut guard)` points nowhere and a `RefMut`'s `truncate(0)` keeps its cell's length (#1885's shape)",
    file,
    find: "                let mut value = self.deep(self.operands(args.iter().map(|a| &a.node)));\n",
    replace: "                let mut value = self.operands(args.iter().map(|a| &a.node));\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_guard_bounds.rs$"],
  },
  {
    name: "effects-rewritten-locals-unescaped",
    breaks:
      "a parameter's field the walk reads as a local of its own, a `RefCell` behind a shared reference, is the body's alone, so a call of the crate's that may change the cell keeps its length",
    file,
    find: "                targets.places.insert(Target::Whole(local), INSIDE);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^refcell_written_through_a_clone_under_its_guard.rs$"],
  },
  {
    name: "effects-constants-point-nowhere",
    breaks:
      "a static's address, `&raw mut STASH`, points nowhere, so a pointer stored into it doesn't escape and a `Vec` cleared through it keeps its length (#1966)",
    file,
    find: "            } else if let Operand::Constant(constant) = operand {\n",
    replace: "            } else if let Operand::Constant(constant) = operand\n                && false\n            {\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_lent_to_code_that_keeps_a_pointer.rs$"],
  },
  {
    name: "effects-indirect-writes-unborrowed",
    breaks:
      "a write through a pointer reaches no whole local, `MaybeBorrowedLocals` taken for none, so `*p = 0` keeps what's known of `n` (#1782)",
    file,
    find: "        Target::Whole(l) => borrowed.is_none_or(|set| set.contains(*l)),\n",
    replace: "        Target::Whole(_) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointer_written_after_its_owner_reads.rs$"],
  },
  {
    name: "effects-replaced-taken-for-items",
    breaks:
      "a write that may replace what it reaches is taken for one of its items, so `*items = Vec::new()` through a guard keeps the cell's length, a write the query no longer sees (#2112 B 3a, the unsound direction)",
    file,
    find: "        WHOLE => Some(Written::Whole),\n",
    replace: "        WHOLE => Some(Written::Items),\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_guard_bounds.rs$"],
  },
  {
    name: "effects-items-taken-for-replaced",
    breaks:
      "a write of a table's items, `board[r][c] = x` or a slice's `swap`, is taken for one that replaces it, so its length is forgotten and each later index checked again (#2112 B 3a, the lossy direction)",
    file,
    find: "        ITEMS => Some(Written::Items),\n",
    replace: "        ITEMS => Some(Written::Whole),\n",
    tests: ["test/corpus.test.ts", "-t", "^table_rows.rs$"],
    snapshots: true,
  },
  {
    name: "effects-references-untyped",
    breaks:
      "a reference to an item or a slice of a `Vec` may replace it, as its type says nothing, so `board[r][c] = x` forgets the table's length and each later index is checked again",
    file,
    find: "                if self.type_of(*t).is_some_and(other) {\n",
    replace: "                if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^table_rows.rs$"],
    snapshots: true,
  },
  {
    name: "effects-scalar-store-resizes-rows",
    breaks:
      "a scalar written through an item's reference may resize a table's rows, so `board[r][c] = x` forgets how long the rows are and `board[r][c - 1]` is checked again",
    file,
    find: "                        written.places.retain(|_, cap| *cap != ITEMS);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^table_rows.rs$"],
    snapshots: true,
  },
  {
    name: "effects-push-replaces",
    breaks:
      "a `push` replaces the `Vec` it's given, so `n < len` past `let n = v.len(); v.push(7)` is forgotten and `v[n]` is checked again",
    file,
    find: "                Written::Grows\n",
    replace: "                Written::Whole\n",
    tests: ["test/corpus.test.ts", "-t", "^length_shifted_past_a_push.rs$"],
    snapshots: true,
  },
  {
    name: "effects-raw-parameters-point-nowhere",
    breaks:
      "a raw pointer a parameter holds points only to its own pointee, so `*p = empty` of a `*mut Vec` into what another parameter's `RefCell` holds keeps the cell's length, and `cell.borrow()[i]` reads past the end where Rust panics",
    file,
    find: "            if outside && crate::lower::mut_refs::holds_a_raw_pointer(self.tcx, ty, &mut Vec::new()) {\n",
    replace: "            if false {\n",
    tests: ["test/bounds-matrix.test.ts", "-t", "shape_unsound_directions"],
  },
  {
    name: "effects-projection-past-what-it-knows",
    breaks:
      "a projection std's named past memory whose pointers the map doesn't know, an `Rc`'s heap, points nowhere, so `v.borrow_mut().truncate(1)` of a `&Rc<RefCell<Vec<u32>>>` keeps the cell's length and `v.borrow()[k]` reads past its end where Rust panics",
    file,
    find: "                                places: IndexMap::from_iter([(t, cap)]),\n",
    replace: "                                places: IndexMap::default(),\n",
    tests: ["test/bounds-matrix.test.ts", "-t", "rc_ref, tested held"],
  },
  {
    name: "effects-std-stores-nothing",
    breaks:
      "std's code that runs none of the crate's stores nothing into what it's given, `v.push(&mut x)`'s `x` not in `v`, so a write through what `v` holds keeps what's known of `x`",
    file,
    find: "                            false => self.deep(\n",
    replace: "                            false => Default::default(),\n                            _ if false => self.deep(\n",
    tests: ["test/bounds-matrix.test.ts", "-t", "shape_std_code_writes"],
  },
  {
    name: "effects-std-stores-what-leads-back",
    breaks:
      "std's code stores into what it's given all it's given reaches, itself too, so `nodes.remove(i)` makes the cell's `Vec` hold pointers to its guard, and a binding given an item of it forgets the guard's length (the lossy direction)",
    file,
    find: "                            false => self.deep(\n",
    replace: "                            false => value.clone(),\n                            _ if false => self.deep(\n",
    tests: ["test/lowering-cell-borrows.test.ts", "-t", "in a borrow's loop is one if"],
  },
  {
    name: "effects-push-into-an-item-grows-the-table",
    breaks:
      "a `push` through a pointer into a table's items, `adj[a].push(x)`, grows the table itself, so dijkstra's `&adj[u]` past `dist[u]` is checked again (the lossy direction)",
    file,
    find: "                        if cap == WHOLE {\n                            grows.insert(t);\n",
    replace: "                        if cap >= ITEMS {\n                            grows.insert(t);\n",
    tests: ["test/snapshots.test.ts", "-t", "^queues: "],
  },
  {
    name: "effects-std-runs-each-type-arguments-code",
    breaks:
      "std's generic code runs the code of each type argument of the crate's, bound or not, so `compare(&heap[p], &heap[i])`'s `index` and the scheduler's min-heap `swap` are checked again (the lossy direction)",
    file,
    find: "            code(t) && (bounded.contains(&i) || crate::lower::analysis::drop_runs_code(tcx, typing_env, t))\n",
    replace: "            code(t) || bounded.contains(&i) && crate::lower::analysis::drop_runs_code(tcx, typing_env, t)\n",
    tests: ["test/lowering-cell-borrows.test.ts", "-t", "in a borrow's loop is one if"],
  },
  {
    name: "effects-bounds-give-no-code",
    breaks:
      "std's generic code runs none of a type argument's, a closure's a `retain` or a `sort_by` is given too, so what the closure clears of a cell is kept",
    file,
    find: "            code(t) && (bounded.contains(&i) || crate::lower::analysis::drop_runs_code(tcx, typing_env, t))\n",
    replace: "            code(t) && (bounded.contains(&usize::MAX) || crate::lower::analysis::drop_runs_code(tcx, typing_env, t))\n",
    tests: ["test/bounds-matrix.test.ts", "-t", "shape_std_code_writes"],
  },
  {
    name: "effects-collections-are-cells",
    breaks:
      "a `Vec<T>` of a type parameter's items is a cell, so a shared reach of it replaces it and the scheduler's `heap` length is forgotten past `compare` (the lossy direction)",
    file,
    find: "is_a_cell(self.tcx, self.typing_env, ty) && !is_a_collection(self.tcx, ty))\n",
    replace: "is_a_cell(self.tcx, self.typing_env, ty) && !is_a_collection(self.tcx, ty) || is_a_collection(self.tcx, ty))\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_fn_ptr_eq.rs$"],
    snapshots: true,
  },
  {
    name: "effects-cell-reference-of-what-it-holds-only",
    breaks:
      "a reference to a cell is taken for one of another type, as only what the cell holds is asked, so `v.borrow_mut().truncate(1)` through `&RefCell` changes its items alone and `v.borrow()[k]` reads past its end where Rust panics",
    file,
    find: "                !levels.contains(&to) && held != to && (!unseen || ty.walk().any(|part| part.as_type() == Some(to)))\n",
    replace: "                held != to && (!unseen || ty.walk().any(|part| part.as_type() == Some(to)))\n",
    tests: ["test/bounds-matrix.test.ts", "-t", "a length of refcell, tested held"],
  },
  {
    name: "effects-rc-reference-unpeeled",
    breaks:
      "a `&RefCell` an `Rc`'s `Deref` gives is taken for one of another type than the `Rc` it's into, so `v.borrow_mut().truncate(1)` of a `&Rc<RefCell<Vec<u32>>>` keeps the cell's length and `v.borrow()[k]` reads past its end where Rust panics",
    file,
    find: "                !levels.contains(&to) && held != to && (!unseen || ty.walk().any(|part| part.as_type() == Some(to)))\n",
    replace: "                ty != to && held != to && (!unseen || ty.walk().any(|part| part.as_type() == Some(to)))\n",
    tests: ["test/bounds-matrix.test.ts", "-t", "rc_ref, tested held"],
  },
  {
    name: "effects-pointer-free-references-reach-on",
    breaks:
      "a reference to what holds no pointer, `&Element`, reaches a cell it's merged with as a shared reach of it, so a binding given an item of a guard's `Vec` replaces the cell (the lossy direction)",
    file,
    find: "            let alone = !pointers_in(self.tcx, self.typing_env, to).holds;\n",
    replace: "            let alone = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_vec_bounds.rs$"],
    snapshots: true,
  },
  {
    name: "effects-slice-pointers-grow",
    breaks:
      "what a pointer to a slice points to may grow, so `addValueToProperties`'s `array[..OBJECT_WIDTH_LIMIT]` past `did_truncate` is checked again (the lossy direction)",
    file,
    find: "                true => extent.min(Written::Items),\n",
    replace: "                true => extent,\n",
    tests: ["test/port-performance-track.test.ts", "-t", "addValueToProperties"],
  },
  {
    name: "effects-named-reads-run-item-code",
    breaks:
      "a read std names, an index of a `Vec`'s or a step of a slice's iterator, runs its items' code where they're the crate's, so `cells[i]` past `cells[i].hits += 1` of a `Vec<Cell2>` is checked again (the lossy direction)",
    file,
    find: "    if named::runs_none_of_what_it_holds(tcx, callee, args) {\n",
    replace: "    if named::runs_none_of_what_it_holds(tcx, callee, args) && false {\n",
    tests: ["test/snapshots.test.ts", "-t", "^values: "],
  },
  {
    name: "effects-projections-reach-all-given",
    breaks:
      "a projection std's named, `&heap[i]`, points to all its arguments reach, the `Vec` and what it holds, so a call given an item writes the `Vec` and the scheduler's min-heap is checked again (the lossy direction)",
    file,
    find: "                    let depth = named::projection_depth(self.tcx, callee, generic)?;\n",
    replace: "                    let depth = named::projection_depth(self.tcx, callee, generic).filter(|_| false)?;\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_items_set_to_a_some_by_index.rs$"],
    snapshots: true,
  },
  {
    name: "effects-rest-arguments-run-code",
    breaks:
      "rust-js's rest arguments carry code, so `value[0]` of a `&dyn Rest<f64>` under `!value.is_empty()` is checked again (the lossy direction)",
    file,
    find: "    let code = |t: Ty<'tcx>| crate::lower::analysis::carries_code(tcx, t, true) && !named::is_a_rest(tcx, t);\n",
    replace: "    let code = |t: Ty<'tcx>| crate::lower::analysis::carries_code(tcx, t, true);\n",
    tests: ["test/rest-parameters.test.ts", "-t", "indexed in bounds are read as a rest slice"],
  },
];
