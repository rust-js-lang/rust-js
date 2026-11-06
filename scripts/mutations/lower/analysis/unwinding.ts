// Mutations of src/lower/analysis/unwinding.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "assert-steps-on",
    breaks:
      "a function whose bound or divisor check may panic is taken for one that never unwinds, so a panic in it skips the drops of what its caller holds",
    file: "src/lower/analysis/unwinding.rs",
    find: "            | TerminatorKind::Return\n",
    replace:
      "            | TerminatorKind::Return\n            | TerminatorKind::Assert { .. }\n",
    tests: ["test/corpus.test.ts", "-t", "^never_unwinding_calls.rs$"],
  },
  {
    name: "user-display-shown-whole",
    breaks:
      "a `Display` of the crate's is taken for one that can't fail, so a function printing one that panics is taken for one that never unwinds",
    file: "src/lower/analysis/unwinding.rs",
    find: "    ty.is_primitive() || ty.is_str() || recognition.is_lang_adt(ty, LangItem::String)\n",
    replace: "    let _ = (ty, recognition);\n    true\n",
    tests: ["test/corpus.test.ts", "-t", "^never_unwinding_calls.rs$"],
  },
  {
    name: "closure-never-candidate",
    breaks:
      "a closure is never one that never unwinds, so a collect of items with a destructor whose closure stage can't panic is `$collectDropping(v.values().map(f), drop)`, where `v.map(f)` would do (#701)",
    file: "src/lower/analysis/unwinding.rs",
    find: "            DefKind::Closure => !tcx.is_coroutine(id),\n",
    replace: "            DefKind::Closure => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_stages_that_never_unwind.rs$"],
    snapshots: true,
  },
  {
    name: "drop-never-steps-on",
    breaks: "a `Drop` that drops nothing is taken to may unwind, so a call of a function that gives a temporary on is wrapped in a `try` that only rethrows (#717)",
    file: "src/lower/analysis/unwinding.rs",
    find: "            TerminatorKind::Drop { .. } => dead.contains(&at),\n",
    replace: "            TerminatorKind::Drop { .. } => false,\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^drops_of_nothing_never_unwind.rs$"],
  },
  {
    name: "cell-set-of-copies-only",
    breaks:
      "a `Cell`'s set of an `Option` of a reference or of an `Rc` no count is kept of is one that may panic, so a guard that restores it drops after its `try` and in a `$cleanup` (#861)",
    file: "src/lower/analysis/unwinding.rs",
    find: "        _ if name == \"set\" && of_a_cell(tcx, callee) => args.types().all(|ty| !drops(ty)),\n",
    replace:
      "        _ if name == \"set\" && of_a_cell(tcx, callee) => args.types().all(|ty| !drops(ty) && ty.is_trivially_pure_clone_copy()),\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_restores_never_unwind.rs$"],
    snapshots: true,
  },
  {
    name: "cell-set-of-what-drops",
    breaks:
      "a `Cell`'s set of a value whose old one has a destructor is taken for one that never panics, so a guard that restores it drops its old value in a `finally` that runs as a panic unwinds, where a panic of that drop must abort (#847, #861)",
    file: "src/lower/analysis/unwinding.rs",
    find: "        _ if name == \"set\" && of_a_cell(tcx, callee) => args.types().all(|ty| !drops(ty)),\n",
    replace: "        _ if name == \"set\" && of_a_cell(tcx, callee) => args.types().all(|ty| !drops(ty) || true),\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_restores_never_unwind.rs$"],
    snapshots: true,
  },
  {
    name: "cell-replace-may-panic",
    breaks:
      "a `Cell`'s replace, which drops nothing, is one that may panic, so a guard whose drop replaces a thread-local drops after its `try` and in a `$cleanup` (#861)",
    file: "src/lower/analysis/unwinding.rs",
    find: "        _ if name == \"replace\" && of_a_cell(tcx, callee) => true,\n",
    replace: "        _ if name == \"replace\" && of_a_cell(tcx, callee) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_restores_never_unwind.rs$"],
    snapshots: true,
  },
  {
    name: "option-take-may-panic",
    breaks:
      "an `Option`'s take is one that may panic, so a guard whose drop restores what it takes, `CURRENT.set(self.0.take())`, drops after its `try` and in a `$cleanup` (#861)",
    file: "src/lower/analysis/unwinding.rs",
    find: "        _ if name == \"take\" && of_an_option(tcx, recognition, callee) => true,\n",
    replace: "        _ if name == \"take\" && of_an_option(tcx, recognition, callee) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_restores_never_unwind.rs$"],
    snapshots: true,
  },
  {
    name: "instance-drops-may-panic",
    breaks:
      "a generic function judged for a use takes each drop of what's of its type parameters, given a value whose drop can't panic, for one that may, so a guard whose drop calls it drops after its `try` and in a `$cleanup` (#861)",
    file: "src/lower/analysis/unwinding.rs",
    find: "                (!(self.may_panic)(ty)).then_some(at)\n",
    replace: "                (!(self.may_panic)(ty) && false).then_some(at)\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_drops_judged_per_use.rs$"],
    snapshots: true,
  },
  {
    name: "instance-calls-unresolved",
    breaks:
      "a generic function judged for a use takes each call of a trait's method, a closure's `call_mut` of one that can't panic, for one that may, so `Defer(|| DEPTH.set(previous))` drops after its `try` and in a `$cleanup` (#861)",
    file: "src/lower/analysis/unwinding.rs",
    find: "                ty::InstanceKind::Item(def) => self.given(def, instance.args, seen),\n",
    replace: "                ty::InstanceKind::Item(def) => def == callee && self.given(def, instance.args, seen),\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_drops_judged_per_use.rs$"],
    snapshots: true,
  },
];
