// Mutations of src/lower/mir/prepared.rs, the MIR a body is lowered from, as it's taken (ADR 0364).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "impl-trait-unrevealed",
    breaks:
      "a body's `impl Trait` stays the opaque type, so what it hides is asked as no iterator: the crate's own iterator given back as `impl DoubleEndedIterator` is closed by a drop given its JS iterator, `TypeError` where Rust counts it, and a peeked `peekable()`'s `enumerate().count()` drops what it peeked at first (#1142)",
    file: "src/lower/mir/prepared.rs",
    find: "        (body, promoted) = revealed::revealed(tcx, (body, promoted));\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "test/crates.test.ts",
      "-t",
      "^((impl_trait_hides_std_adapters|impl_trait_hides_own_iterator|rc_of_impl_fn_returned|closures_made_and_folded|returned_impl_ends|lazy_iterator_clone|consumed_own_iterator_dropped_once|try_folds_of_stages_unwind).rs|two crates: impl_iterator_of_a_library)$",
    ],
  },
  {
    name: "optimized-boxes-read-through-pointer",
    breaks:
      "a library's generic body, from its optimized MIR, reads a box through the pointer rustc makes of it, which its consumer refuses (#378)",
    file: "src/lower/mir/prepared.rs",
    find: "            boxes_read_as_values(tcx, &mut body);\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: generic_boxed_drop"],
  },
  {
    name: "mir-await-loop-kept",
    breaks:
      "an `.await`'s poll loop is lowered as it is, its `yield` none JS has",
    file: "src/lower/mir/prepared.rs",
    find: "            true => awaits::awaits(tcx, &mut body),\n",
    replace: "            true => Default::default(),\n",
    tests: [
      "test/lowering-functions.test.ts",
      "test/lowering-matches.test.ts",
      "-t",
      "an awaited reference given to a generic function is awaited|an async fn",
    ],
  },
  {
    name: "mir-cannot-throw-unwinds",
    breaks:
      "MIR's call of what can't throw cleans up, its library function taking a drop",
    file: "src/lower/mir/prepared.rs",
    find: "                *unwind = UnwindAction::Unreachable;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(drop_temporaries|allocator|refcell_vec_bounds).rs$"],
    snapshots: true,
  },
  {
    name: "dead-switch-kept",
    breaks:
      "a std body's switch that goes one way whatever it tests, which drop elaboration leaves, is kept, and its discriminant read as a value",
    file: "src/lower/mir/prepared.rs",
    find: "                    if let [way] = ways[..] {\n",
    replace:
      "                    if let [way] = ways[..]\n                        && false\n                    {\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "dead-discriminant-kept",
    breaks:
      "the discriminant a std body read for a switch that went is kept, read as a value",
    file: "src/lower/mir/prepared.rs",
    find: "!read.0.contains(&local)))\n",
    replace: "false))\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "assume-kept",
    breaks:
      "an `assume` of what an inlined `?` knows of its residual is kept in a std body, and the discriminant it compares read as a value",
    file: "src/lower/mir/prepared.rs",
    find: "                        if matches!(**intrinsic, mir::NonDivergingIntrinsic::Assume(_)))\n",
    replace:
      "                        if matches!(**intrinsic, mir::NonDivergingIntrinsic::Assume(_)) && false)\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "called-once-drops-nothing-returning",
    breaks:
      "a closure a binding calls once, rustc's `Fn`, drops nothing of what it took as its call returns, where Rust's `call_once` drops it (ADR 1020)",
    file: "src/lower/mir/prepared.rs",
    find: "                TerminatorKind::Return => *kind = dropped(returned, UnwindAction::Continue),\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^called_once_closure_drops_after.rs$",
    ],
  },
  {
    name: "called-once-own-cleanup-resumes",
    breaks:
      "a closure a binding calls once, rustc's `Fn`, whose body drops a value of its own as it unwinds, resumes after it, dropping nothing of what it took (ADR 1020)",
    file: "src/lower/mir/prepared.rs",
    find: "                TerminatorKind::UnwindResume => *kind = TerminatorKind::Goto { target: unwound },\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^called_once_closure_drops_after.rs$",
    ],
  },
  {
    name: "called-once-self-left-a-reference",
    breaks:
      "a closure a binding calls once, taken by value, reads what it took through itself as a reference still (ADR 1020)",
    file: "src/lower/mir/prepared.rs",
    find: "                        place.projection = self.tcx.mk_place_elems(&place.projection[1..]);\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^called_once_closure_drops_after.rs$",
    ],
  },
  {
    name: "unreachable-hint-a-call",
    breaks:
      "`hint::unreachable_unchecked()` stays a call, not MIR's unreachable code, so a `match`'s arm of only it keeps its test and a throw (rd643)",
    file: "src/lower/mir/prepared.rs",
    find: "                data.terminator_mut().kind = TerminatorKind::Unreachable;\n            }\n        }\n        // An optimized body's",
    replace: "            }\n        }\n        // An optimized body's",
    tests: ["test/corpus.test.ts", "-t", "^unreachable_hint.rs$"],
    snapshots: true,
  },
  {
    name: "box-copied-from-a-place-read-as-the-copy",
    breaks: "a library's impl the consumer compiles, of optimized MIR that reads a box through a copy of it, `_b = copy (*_r)`, gives a `&mut` to that copy, so a write through it is lost (#1455)",
    file: "src/lower/mir/prepared.rs",
    find: "                Some((_, from)) => from,\n",
    replace: "                Some(_) => Place::from(boxed),\n",
    tests: ["test/crates.test.ts", "-t", "boxed_places_a_consumer_writes_through"],
  },
  {
    name: "box-copy-only-just-before",
    breaks: "a box copied out of a place earlier in the block than just before its pointer, `**b += 1`'s two copies, isn't read through that place, the write lost (#1455)",
    file: "src/lower/mir/prepared.rs",
    find: "                [copy_at] if copy_at.statement_index < at => {\n",
    replace: "                [copy_at] if copy_at.statement_index + 1 == at => {\n",
    tests: ["test/crates.test.ts", "-t", "boxed_places_a_consumer_writes_through"],
  },
  {
    name: "discriminant-read-by-a-call-dropped",
    breaks: "a discriminant read by its block's call alone, 1.100's derived `Hash`'s `write_isize(state, discr)`, is dropped as unread: std's `IpAddr` hash reads a name nothing declares (ADR 0109, amended)",
    file: "src/lower/mir/prepared.rs",
    find: "                Visitor::visit_terminator(&mut read, data.terminator(), mir::Location::START);\n",
    replace: "",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "std-question-by-name",
    breaks: "a std call named `len`, `is_empty` or the like is taken never to unwind by its name, so `Range::is_empty` of an order that panics drops nothing as it unwinds (#1666)",
    file: "src/lower/mir/prepared.rs",
    find: "                        crate::lower::analysis::std_call_never_unwinds(tcx, typing_env, (id, args))\n",
    replace: "                        true\n",
    tests: ["test/corpus.test.ts", "-t", "^range_(inclusive_)?is_empty_of_a_panicking_order_unwinds.rs$"],
  },
];
