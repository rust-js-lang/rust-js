// Mutations of src/lower/analysis/effects.rs, what a statement may write,
// the query the MIR intervals' facts are kept by (#2112 B). Each makes the
// query keep a fact a write reaches, which its shadow's row sees.
import type { Mutation } from "../../../mutations";

const file = "src/lower/analysis/effects.rs";
const tests = ["test/intervals-shadow.test.ts", "-t", "keeps nothing the forgetting rules forget"];
// What the transfer that asks the query proves of what no write reaches.
const proofs = ["test/intervals-shadow.test.ts", "-t", "keeps what the rules prove"];

export const mutations: Mutation[] = [
  {
    name: "effects-call-result-shallow",
    breaks:
      "a call's result holds only the pointers it's given, not what they reach, so `deref_mut(&mut guard)` points nowhere and a `RefMut`'s `truncate(0)` keeps its cell's length (#1885's shape)",
    file,
    find: "                let mut value = self.deep(self.operands(args.iter().map(|a| &a.node)));\n",
    replace: "                let mut value = self.operands(args.iter().map(|a| &a.node));\n",
    tests,
  },
  {
    name: "effects-rewritten-locals-unescaped",
    breaks:
      "a parameter's field the walk reads as a local of its own, a `RefCell` behind a shared reference, is the body's alone, so a call of the crate's that may change the cell keeps its length",
    file,
    find: "                targets.places.insert(Target::Whole(local), INSIDE);\n",
    replace: "",
    tests,
  },
  {
    name: "effects-constants-point-nowhere",
    breaks:
      "a static's address, `&raw mut STASH`, points nowhere, so a pointer stored into it doesn't escape and a `Vec` cleared through it keeps its length (#1966)",
    file,
    find: "            } else if let Operand::Constant(constant) = operand {\n",
    replace: "            } else if let Operand::Constant(constant) = operand\n                && false\n            {\n",
    tests,
  },
  {
    name: "effects-borrows-by-lifetime-hold-nothing",
    breaks:
      "std's type that holds a borrow by a lifetime, a `RefMut`, holds no pointer, so what a guard lends points nowhere and `*items = Vec::new()` keeps the cell's length (#1928's shape)",
    file,
    find: "                if args.regions().next().is_some() {\n",
    replace: "                if false {\n",
    tests,
  },
  {
    name: "effects-box-holds-nothing",
    breaks:
      "a `Box` holds no pointer, so a closure's capture of `&(*items)`, the cell in the box, points nowhere and calling it keeps the cell's length",
    file,
    find: "                if raw_pointee(tcx, ty).is_some() || ty.is_box() {\n",
    replace: "                if raw_pointee(tcx, ty).is_some() {\n",
    tests,
  },
  {
    name: "effects-nonnull-holds-nothing",
    breaks:
      "`NonNull<Vec<u32>>` holds no pointer, as its type arguments say, so `NonNull::from(&mut n)` stored away doesn't escape and a call that clears `n` through it keeps its length (#1966)",
    file,
    find: "                if raw_pointee(tcx, ty).is_some() || ty.is_box() {\n",
    replace: "                if ty.is_box() {\n",
    tests,
  },
  {
    name: "effects-shared-cells-unwritable",
    breaks:
      "a shared reference to a cell can't change it, `Freeze` unasked, so `borrow_mut()` of a `&RefCell` gives a guard that writes nothing and a `truncate` through it keeps the cell's length",
    file,
    find: "                if !ty.is_freeze(tcx, typing_env) || raw_pointee(tcx, ty).is_some() {\n",
    replace: "                if raw_pointee(tcx, ty).is_some() {\n",
    tests,
  },
  {
    name: "effects-indirect-writes-unborrowed",
    breaks:
      "a write through a pointer reaches no whole local, `MaybeBorrowedLocals` taken for none, so `*p = 0` keeps what's known of `n` (#1782)",
    file,
    find: "        Target::Whole(l) => borrowed.is_none_or(|set| set.contains(*l)),\n",
    replace: "        Target::Whole(_) => false,\n",
    tests,
  },
  {
    name: "effects-replaced-taken-for-items",
    breaks:
      "a write that may replace what it reaches is taken for one of its items, so `*items = Vec::new()` through a guard keeps the cell's length, a write the query no longer sees (#2112 B 3a, the unsound direction)",
    file,
    find: "        WHOLE => Some(Written::Whole),\n",
    replace: "        WHOLE => Some(Written::Items),\n",
    tests,
  },
  {
    name: "effects-items-taken-for-replaced",
    breaks:
      "a write of a table's items, `board[r][c] = x` or a slice's `swap`, is taken for one that replaces it, so its length is forgotten and each later index checked again (#2112 B 3a, the lossy direction)",
    file,
    find: "        ITEMS => Some(Written::Items),\n",
    replace: "        ITEMS => Some(Written::Whole),\n",
    tests: proofs,
  },
  {
    name: "effects-references-untyped",
    breaks:
      "a reference to an item or a slice of a `Vec` may replace it, as its type says nothing, so `board[r][c] = x` forgets the table's length and each later index is checked again",
    file,
    find: "                if !cell && of.is_some_and(|ty| ty != to) {\n",
    replace: "                if false {\n",
    tests: proofs,
  },
  {
    name: "effects-scalar-store-resizes-rows",
    breaks:
      "a scalar written through an item's reference may resize a table's rows, so `board[r][c] = x` forgets how long the rows are and `board[r][c - 1]` is checked again",
    file,
    find: "                        written.places.retain(|_, cap| *cap != ITEMS);\n",
    replace: "",
    tests: proofs,
  },
  {
    name: "effects-push-replaces",
    breaks:
      "a `push` replaces the `Vec` it's given, so `n < len` past `let n = v.len(); v.push(7)` is forgotten and `v[n]` is checked again",
    file,
    find: "                Written::Grows\n",
    replace: "                Written::Whole\n",
    tests: proofs,
  },
  {
    name: "rc-deref-writes-cells",
    breaks: "an `Rc`'s `Deref` is taken as a call that may write a cell, forgetting its length, so React's `listeners.borrow()[i]` keeps its check, a call-bounds line (#1839)",
    file,
    find: "        || rc_deref(tcx, callee, args)\n",
    replace: "",
    tests: ["test/intervals-shadow.test.ts", "-t", "gives its cell, the same each time"],
  },
  {
    name: "range-step-writes-cells",
    breaks: "a `for` loop's `next()` of a range, std's, is taken to may write a cell, so `batch.borrow()[i]` of `i` in `(0..batch.borrow().len()).step_by(2)` keeps its check (ADR 1168, #1948)",
    file,
    find: "        || range_step(tcx, callee, args).is_some()\n",
    replace: "",
    tests: ["test/lowering-cell-flow.test.ts", "-t", "a held borrow counts the cells"],
  },
  {
    name: "any-call-reads-cells",
    breaks: "any call is taken to write no cell, so past a held length's `borrow_mut().clear()` in the loop `batch.borrow()[i]` reads past the end, `undefined` where Rust panics (ADR 1168, #1948)",
    file,
    find: "        || range_step(tcx, callee, args).is_some()\n",
    replace: "        || true\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_length_across_a_loops_steps.rs$"],
  },
];
