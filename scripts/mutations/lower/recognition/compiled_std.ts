// Mutations of crates/rust-js-lower/src/recognition/compiled_std.rs, the catalog of std's
// compiled bodies (split from std_items.rs, #1156).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "meaning-only-made-everywhere",
    breaks:
      "a `MEANING_ONLY` adapter, `Filter`, is std's struct in the normal build too, so `v.into_iter().filter(p).collect()` isn't the JS chain the recognizers make (ADR 0932)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "    !MEANING_ONLY.contains(&item) || crate::session::caches(tcx).meaning.get()\n",
    replace:
      "    true || !MEANING_ONLY.contains(&item) || crate::session::caches(tcx).meaning.get()\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_given_back_stepped_from_its_back.rs$"],
    snapshots: true,
  },
  {
    name: "meaning-only-library-items-seen",
    breaks:
      "the std manifest's `Filter` impls and methods are taken for the normal build's to call though it doesn't compile `Filter`: `filter(..).collect()` is refused, `this implementation` (ADR 0932)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "    if crate::session::caches(tcx).meaning.get() || id.is_local() {\n        return false;\n    }\n",
    replace:
      "    if true || crate::session::caches(tcx).meaning.get() || id.is_local() {\n        return false;\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_given_back_stepped_from_its_back.rs$"],
    snapshots: true,
  },
  {
    name: "private-callee-not-compiled",
    breaks:
      "a function std keeps private that a compiled body calls, `Fuse`'s `and_then_or_clear`, isn't compiled with it, and the call is refused",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "                    && ((!tcx.visibility(called).is_public()\n",
    replace:
      "                    && ((!tcx.visibility(called).is_public()\n                        && false\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "named-private-fns-uncompiled",
    breaks: "a function std keeps private that a compiled body names as a value, `FlattenCompat::last`'s own `last`, isn't compiled with it, so `Flatten::last` isn't (#645)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "        for value in named_fns(body, &blocks) {\n",
    replace: "        for value in Vec::<DefId>::new() {\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "flatten-count-uncompiled",
    breaks: "std's `Flatten::count` isn't compiled, so a held one, and every one in the build of std's structs, is counted by std's default, each inner iterator stepped by `next`, not by its own `count` (#736)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "    (StdPath::IterFlatMap, Some(\"Iterator\"), \"count\"),\n",
    replace: "",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "io-lines-uncompiled",
    breaks: "std's `Lines` iterator isn't compiled, so `lines()` is no iterator (#1156)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "    (StdPath::IoLines, Some(\"Iterator\"), \"next\"),\n",
    replace: "",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "zip-clone-uncompiled",
    breaks:
      "a held zip's `clone()` isn't std's compiled `Zip::clone`: the std package isn't what its catalog makes, and with std's structs alone it's refused (ADR 0975)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: '    (StdPath::IterZip, Some("Clone"), "clone"),\n',
    replace: "",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "copied-back-uncompiled",
    breaks:
      "std's `Copied`'s `next_back` isn't compiled: the build of std's structs alone refuses a `copied()` stepped from its back, `the DoubleEndedIterator of a Copied<slice::Iter>` (ADR 0938)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: '    (StdPath::IterCopied, Some("DoubleEndedIterator"), "next_back"),\n',
    replace: "",
    tests: [
      "test/iterator-forms.test.ts",
      "-t",
      "^(copied_structs_apart|associated_double_ended) with",
    ],
  },
  {
    name: "cloned-reader-uncompiled",
    breaks:
      "std's `Cloned`'s `__iterator_get_unchecked` isn't compiled: the build of std's structs alone refuses a `cloned()` std's zip reads by index, `reading a Cloned<slice::Iter<D>> by its index` (#727, ADR 0938)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: '    (StdPath::IterCloned, Some("Iterator"), "__iterator_get_unchecked"),\n',
    replace: "",
    tests: [
      "test/iterator-forms.test.ts",
      "-t",
      "^adapters_own_work_that_panics with",
    ],
  },
  {
    name: "cloned-compiled-everywhere",
    breaks:
      "std's `Cloned` isn't `MEANING_ONLY`, so the normal build makes a held `cloned()` std's struct, not the JS iterator the recognizers make of it: the normal build's JS changes (#727, ADR 0938)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "    // rows ITERATOR_TABLE=copies ITERATOR_MATRIX=/[ .]cloned[ .|]/.\n    StdPath::IterCloned,\n",
    replace: "    // rows ITERATOR_TABLE=copies ITERATOR_MATRIX=/[ .]cloned[ .|]/.\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^cloned_items_from_both_ends.rs$"],
  },
  {
    name: "filter-meaning-unmade",
    breaks:
      "std's `Filter` isn't made as its meaning: the build of std's structs alone steps a `rev()` through the recognizers' `filter`, `it.back is not a function` (ADR 0932)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: '    ("Iterator", "enumerate"),\n    ("Iterator", "map"),\n    ("Iterator", "filter"),\n',
    replace: '    ("Iterator", "enumerate"),\n    ("Iterator", "map"),\n',
    tests: ["test/compiled-std.test.ts", "-t", "the catalogue kept is the walk's of each of its groups"],
  },
  {
    name: "flat-map-not-held",
    breaks:
      "a `flat_map()` a variable holds is the recognizers' JS iterator, sized `(0, None)` where std's counts",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: '    (StdPath::IterFlatMap, "flat_map", false),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^std_flat_map.rs$"],
    snapshots: true,
  },
  {
    name: "take-not-held",
    breaks:
      "a held `take()` is the recognizers' JS iterator, not std's `Take`, which knows its `size_hint`",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: '    (StdPath::IterTake, "take", true),\n',
    replace: "",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^held_skip_take.rs$"],
  },
  {
    name: "step-by-not-held",
    breaks:
      "a held `step_by()` is the recognizers' JS iterator, not std's `StepBy`",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: '    (StdPath::IterStepBy, "step_by", false),\n',
    replace: "",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_nth_fold.rs$"],
  },
  {
    name: "chain-not-held",
    breaks:
      "a held `chain()` is the recognizers' JS iterator, not std's `Chain`",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: '    (StdPath::IterChain, "chain", false),\n',
    replace: "",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
  },
  {
    name: "zip-sized-in-place",
    breaks:
      "a zip whose `size_hint()` is asked is made in place, `$lazyZip`, whose sides, JS's own, count nothing: `(0, Some(5))` of `v.iter().zip(v.iter().skip(2))` (#400)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: '    (StdPath::IterZip, "zip", false),\n',
    replace: '    (StdPath::IterZip, "zip", true),\n',
    tests: ["test/corpus.test.ts", "-t", "^zip_size_hints.rs$"],
  },
  {
    name: "io-chain-fn-not-compiled",
    breaks: "`core::io::chain`, which `Read::chain` calls and std keeps public but unstable, isn't compiled, so `@rust-js/std` can't be built (#1156)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: '    (StdPath::IoChainFn, None),\n',
    replace: "",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "question-mark-try-compiled",
    breaks: "a `?` of a `Result` in a compiled body, `Chain::read`'s, has std's `Try::branch` compiled for it, which the lowering writes itself, and `@rust-js/std` can't be built (#1156)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "            if super::is_question_mark_written(tcx, callee, args) {\n",
    replace: "            if false && super::is_question_mark_written(tcx, callee, args) {\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "debug-shown-unnoted",
    breaks: "a type of std's adapter compiled as its meaning alone whose `{:?}` the crate asks isn't noted, so it stays the recognizers' and its `{:?}` is refused in the plain build (#1401)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "                        types.insert(session_key(tcx, part));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^debug_of_adapters_compiled_as_their_meaning.rs$"],
  },
  {
    name: "debug-shown-type-not-compiled",
    breaks: "a type whose `{:?}` the crate asks isn't std's struct where it's held, its adapter's name alone answering, so its `{:?}` is refused (#1401)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "                    .is_some_and(|(types, _)| types.contains(&session_key(tcx, ty)))\n",
    replace: "                    .is_some_and(|(types, _)| types.is_empty() && types.contains(&session_key(tcx, ty)))\n",
    tests: ["test/corpus.test.ts", "-t", "^debug_of_adapters_compiled_as_their_meaning.rs$"],
  },
  {
    name: "debug-shown-maker-of-any-type",
    breaks: "a maker of std's adapter compiled as its meaning alone, `filter`, is called compiled whatever type it makes, so a filter whose `{:?}` nothing asks is std's struct too, `iterator_filter(..)` where it was `range.filter(p)` (#1401)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "        return compiled_for(tcx, made);\n",
    replace: "        return true;\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_of_a_compiled_adapter_knowing_its_back.rs$"],
    snapshots: true,
  },
  {
    name: "repeat-n-maker-uncompiled",
    breaks:
      "std's `repeat_n` isn't compiled, so the build of std's bodies makes the runtime's `$repeatingN` and pits nothing against it (#1458)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "    (StdPath::IterRepeatNFn, Some(StdPath::IterRepeatN)),\n",
    replace: "",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std's bodies as the compiler compiles them"],
  },
  {
    name: "repeat-n-maker-called-plain",
    breaks:
      "the plain build calls std's compiled `repeat_n`, `repeatN(L(7), 3, clone)`, not the runtime's `$repeatingN(L(7), 3)`, though `RepeatN` is compiled as its meaning alone (#1458)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "makes.is_some_and(not_called) && fn_id == id",
    replace: "false && fn_id == id",
    tests: ["test/corpus.test.ts", "-t", "^repeat_n_drops_what_it_holds.rs$"],
    snapshots: true,
  },
  {
    name: "repeat-n-compiled-in-every-build",
    breaks:
      "`RepeatN` is compiled for every build, so the plain build makes std's struct of `repeat_n(x, n)`, not the runtime's `$repeatingN(x, n)` (#1458)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "    StdPath::IterRepeatN,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^repeat_n_drops_what_it_holds.rs$"],
    snapshots: true,
  },
  {
    name: "cycle-maker-uncompiled",
    breaks:
      "std's `Iterator::cycle` isn't compiled, so the build of std's bodies makes the runtime's `$cycleOf` and pits nothing against it (#1458)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: '    ("Iterator", "cycle"),\n',
    replace: "",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std's bodies as the compiler compiles them"],
  },
  {
    name: "cycle-maker-called-plain",
    breaks:
      "the plain build calls std's compiled `cycle`, not the runtime's `$cycleOf`, though `Cycle` is compiled as its meaning alone (#1458)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: '    (StdPath::IterCycle, "cycle", false),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^cycle_drops_what_it_replaces.rs$"],
    snapshots: true,
  },
  {
    name: "cycle-meaning-only",
    breaks:
      "`Cycle` is `MEANING_ONLY` again, so a `cycle()` held apart is the runtime's `$cycleOf`, not std's compiled struct (#1681's change 2, the cycle family)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "    StdPath::IterCopied,\n",
    replace: "    StdPath::IterCopied,\n    StdPath::IterCycle,\n",
    tests: ["test/corpus.test.ts", "-t", "^held_cycle.rs$"],
    snapshots: true,
  },
  {
    name: "filter_map-meaning-only",
    breaks:
      "`FilterMap` is `MEANING_ONLY` again, so a `filter_map()` held apart, stepped from its back, is the recognizers' `$filterEnds` of a `$mapEnds`, not std's compiled struct (#1681's change 2, the filter_map family)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "    StdPath::IterCopied,\n",
    replace: "    StdPath::IterCopied,\n    StdPath::IterFilterMap,\n",
    tests: ["test/corpus.test.ts", "-t", "^(held_filter_map_from_its_back|held_filter_map_reversed).rs$"],
    snapshots: true,
  },
  {
    name: "take_while-meaning-only",
    breaks:
      "`TakeWhile` is `MEANING_ONLY` again, so a `take_while()` made apart is the runtime's `$lazyTakeWhile`, not std's compiled struct (#1681's change 2, the take_while family)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "    StdPath::IterCopied,\n",
    replace: "    StdPath::IterTakeWhile,\n    StdPath::IterCopied,\n",
    tests: ["test/corpus.test.ts", "-t", "^(held_lazy_adapters_hint|impl_trait_hides_std_adapters|own_iterator_holding_a_closure_dropped_by_generic_code).rs$"],
    snapshots: true,
  },
  {
    name: "skip_while-meaning-only",
    breaks:
      "`SkipWhile` is `MEANING_ONLY` again, so a `skip_while()` made apart is the runtime's `$lazySkipWhile`, not std's compiled struct (#1681's change 2, the skip_while family)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "    StdPath::IterCopied,\n",
    replace: "    StdPath::IterSkipWhile,\n    StdPath::IterCopied,\n",
    tests: ["test/corpus.test.ts", "-t", "^(held_lazy_adapters_hint|size_hint_of_adapters).rs$"],
    snapshots: true,
  },
  {
    name: "map_while-meaning-only",
    breaks:
      "`MapWhile` is `MEANING_ONLY` again, so a `map_while()` made apart is the runtime's `$lazyMapWhile`, not std's compiled struct (#1681's change 2, the map_while family)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "    StdPath::IterCopied,\n",
    replace: "    StdPath::IterMapWhile,\n    StdPath::IterCopied,\n",
    tests: ["test/corpus.test.ts", "-t", "^(held_lazy_adapters_hint|size_hint_of_adapters).rs$"],
    snapshots: true,
  },
  {
    name: "scan-meaning-only",
    breaks:
      "`Scan` is `MEANING_ONLY` again, so a `scan()` made apart is the runtime's `$lazyScan`, not std's compiled struct (#1681's change 2, the scan family)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "    StdPath::IterCopied,\n",
    replace: "    StdPath::IterScan,\n    StdPath::IterCopied,\n",
    tests: ["test/corpus.test.ts", "-t", "^(scan_map_while_unfused|size_hint_of_adapters).rs$"],
    snapshots: true,
  },
  {
    name: "take-advance-back-by-uncompiled",
    breaks:
      "std's `Take::advance_back_by` isn't compiled, so a compiled `take(n).rev()` counted runs the trait's default, its source's `nth_back` one at a time, not one `advance_back_by` that drops a `Vec`'s as one slice (#2125)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: '    (StdPath::IterTake, Some("DoubleEndedIterator"), "advance_back_by"),\n',
    replace: "",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "repeat-nth-uncompiled",
    breaks:
      "std's `Repeat::nth` isn't compiled, so std's compiled `Repeat` steps by the trait's default `nth`, cloning `x` at each step it passes, where std's clones it once (#1569)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: '    (StdPath::IterRepeat, Some("Iterator"), "nth"),\n',
    replace: "",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "private-debug-shown-uncompiled",
    breaks: "a `Flatten`'s compiled `Debug` writes its private `FlattenCompat`, whose `Debug` the catalogue's walk leaves out, as the `&dyn Debug` it's given is no call: `@rust-js/std`'s build refuses it (#2252)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "        found.extend(shown_private(tcx, typing_env, body, &blocks));\n",
    replace: "        let _ = shown_private(tcx, typing_env, body, &blocks);\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std's bodies as the compiler compiles them"],
  },
  {
    name: "successors-debug-uncompiled",
    breaks: "`{:?}` of a `Successors` is refused: std's `Debug` of it isn't compiled (#2252)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "    (StdPath::IterSuccessors, Some(\"Debug\"), \"fmt\"),\n",
    replace: "",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std's bodies as the compiler compiles them"],
  },
  {
    name: "successors-maker-uncompiled",
    breaks: "`successors(..)` whose `{:?}` is asked is made the recognizers' `$successors`, not std's struct, so it's refused (#2252)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "    (StdPath::IterSuccessorsFn, Some(StdPath::IterSuccessors)),\n",
    replace: "",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std's bodies as the compiler compiles them"],
  },
  {
    name: "successors-held-everywhere",
    breaks: "every `successors(..)` is std's struct, not only where `{:?}` is asked of it, its JS `iterator_successors(..)` where it was `$successors(..)` (ADR 0932)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "    StdPath::IterSuccessors,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^iterator_sources.rs$"],
    snapshots: true,
  },
  {
    name: "partition-of-vec-sides-compiled",
    breaks:
      "every `partition` is std's compiled body, a `Vec`'s sides too, `iterator_partition(.., vecDefault, vecExtend)` where its helper's `$partition` fills them as std does (#2387)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "        _ if id == compiled_provided_method(tcx, \"Iterator\", \"partition\") => args.types().nth(1).is_some_and(filled),\n",
    replace: "        _ if false && id == compiled_provided_method(tcx, \"Iterator\", \"partition\") => args.types().nth(1).is_some_and(filled),\n",
    tests: ["test/corpus.test.ts", "-t", "^consumers_step_each.rs$"],
    snapshots: true,
  },
  {
    name: "unzip-helper-of-mixed-sides",
    breaks: "`unzip()` into a crate's `Extend` beside a `Vec` is the helper's, whose arrays never call the crate's `extend` (#2395)",
    file: "crates/rust-js-lower/src/recognition/compiled_std.rs",
    find: "        _ if id == compiled_provided_method(tcx, \"Iterator\", \"unzip\") => args.types().skip(3).all(filled),\n",
    replace: "        _ if id == compiled_provided_method(tcx, \"Iterator\", \"unzip\") => args.types().skip(3).any(filled),\n",
    tests: ["test/corpus.test.ts", "-t", "^unzip_fills_each_side_as_std_extends_it.rs$"],
  },
];
