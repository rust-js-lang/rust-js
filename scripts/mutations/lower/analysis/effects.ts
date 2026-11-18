// Mutations of src/lower/analysis/effects.rs, what a statement may write,
// the query the MIR intervals' facts are kept by (#2112 B). Each makes the
// query keep a fact a write reaches, which its shadow's row sees.
import type { Mutation } from "../../../mutations";

const file = "src/lower/analysis/effects.rs";
const tests = ["test/intervals-shadow.test.ts", "-t", "keeps nothing the forgetting rules forget"];

export const mutations: Mutation[] = [
  {
    name: "effects-call-result-shallow",
    breaks:
      "a call's result holds only the pointers it's given, not what they reach, so `deref_mut(&mut guard)` points nowhere and a `RefMut`'s `truncate(0)` keeps its cell's length (#1885's shape)",
    file,
    find: "                let value = self.deep(self.operands(args.iter().map(|a| &a.node)));\n",
    replace: "                let value = self.operands(args.iter().map(|a| &a.node));\n",
    tests,
  },
  {
    name: "effects-rewritten-locals-unescaped",
    breaks:
      "a parameter's field the walk reads as a local of its own, a `RefCell` behind a shared reference, is the body's alone, so a call of the crate's that may change the cell keeps its length",
    file,
    find: "                escapes.entry(start).or_default().insert(Target::Whole(local));\n",
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
    find: "            Target::Whole(l) => borrowed.is_none_or(|set| set.contains(*l)),\n",
    replace: "            Target::Whole(_) => false,\n",
    tests,
  },
];
