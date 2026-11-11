// Mutations of src/lower/recognition/compiled_std.rs, the catalog of std's
// compiled bodies (split from std_items.rs, #1156).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "meaning-only-made-everywhere",
    breaks:
      "a `MEANING_ONLY` adapter, `Filter`, is std's struct in the normal build too, so `v.into_iter().filter(p).collect()` isn't the JS chain the recognizers make (ADR 0932)",
    file: "src/lower/recognition/compiled_std.rs",
    find: "    !MEANING_ONLY.contains(&type_name) || crate::lower::session::caches(tcx).meaning.get()\n",
    replace:
      "    true || !MEANING_ONLY.contains(&type_name) || crate::lower::session::caches(tcx).meaning.get()\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^(collect_drops|filter_ends).rs$"],
  },
  {
    name: "meaning-only-library-items-seen",
    breaks:
      "the std manifest's `Filter` impls and methods are taken for the normal build's to call though it doesn't compile `Filter`: `filter(..).collect()` is refused, `this implementation` (ADR 0932)",
    file: "src/lower/recognition/compiled_std.rs",
    find: "    if crate::lower::session::caches(tcx).meaning.get() || id.is_local() {\n        return false;\n    }\n",
    replace:
      "    if true || crate::lower::session::caches(tcx).meaning.get() || id.is_local() {\n        return false;\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^(collect_drops|filter_ends).rs$"],
  },
  {
    name: "private-callee-not-compiled",
    breaks:
      "a function std keeps private that a compiled body calls, `Fuse`'s `and_then_or_clear`, isn't compiled with it, and the call is refused",
    file: "src/lower/recognition/compiled_std.rs",
    find: "                    && ((!tcx.visibility(called).is_public()\n",
    replace:
      "                    && ((!tcx.visibility(called).is_public()\n                        && false\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "named-private-fns-uncompiled",
    breaks: "a function std keeps private that a compiled body names as a value, `FlattenCompat::last`'s own `last`, isn't compiled with it, so `Flatten::last` isn't (#645)",
    file: "src/lower/recognition/compiled_std.rs",
    find: "        for value in named_fns(tcx, body) {\n",
    replace: "        for value in Vec::<DefId>::new() {\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "flatten-count-uncompiled",
    breaks: "std's `Flatten::count` isn't compiled, so a held one, and every one in the build of std's structs, is counted by std's default, each inner iterator stepped by `next`, not by its own `count` (#736)",
    file: "src/lower/recognition/compiled_std.rs",
    find: "    (\"FlatMap\", Some(\"Iterator\"), \"count\"),\n",
    replace: "",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "io-lines-uncompiled",
    breaks: "std's `Lines` iterator isn't compiled, so `lines()` is no iterator (#1156)",
    file: "src/lower/recognition/compiled_std.rs",
    find: "    (\"Lines\", Some(\"Iterator\"), \"next\"),\n",
    replace: "",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "zip-clone-uncompiled",
    breaks:
      "a held zip's `clone()` isn't std's compiled `Zip::clone`: the std package isn't what its catalog makes, and with std's structs alone it's refused (ADR 0975)",
    file: "src/lower/recognition/compiled_std.rs",
    find: '    ("Zip", Some("Clone"), "clone"),\n',
    replace: "",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "copied-back-uncompiled",
    breaks:
      "std's `Copied`'s `next_back` isn't compiled: the build of std's structs alone refuses a `copied()` stepped from its back, `the DoubleEndedIterator of a Copied<slice::Iter>` (ADR 0938)",
    file: "src/lower/recognition/compiled_std.rs",
    find: '    ("Copied", Some("DoubleEndedIterator"), "next_back"),\n',
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
    file: "src/lower/recognition/compiled_std.rs",
    find: '    ("Cloned", Some("Iterator"), "__iterator_get_unchecked"),\n',
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
    file: "src/lower/recognition/compiled_std.rs",
    find: '    "Copied",\n    "Cloned",\n];\n',
    replace: '    "Copied",\n];\n',
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^cloned_items_from_both_ends.rs$"],
  },
  {
    name: "filter-meaning-unmade",
    breaks:
      "std's `Filter` isn't made as its meaning: the build of std's structs alone steps a `rev()` through the recognizers' `filter`, `it.back is not a function` (ADR 0932)",
    file: "src/lower/recognition/compiled_std.rs",
    find: '    ("Iterator", "filter"),\n',
    replace: "",
    tests: ["test/iterator-forms.test.ts", "-t", "^rev_of_lazy_chains with"],
  },
  {
    name: "flatten-sized-in-place",
    breaks:
      "`size_hint()` of a flatten made in place is the recognizers', `(0, None)` of arrays' items",
    file: "src/lower/recognition/compiled_std.rs",
    find: '    ("Flatten", "flatten", false),\n',
    replace: '    ("Flatten", "flatten", true),\n',
    tests: ["test/corpus.test.ts", "-t", "^std_flatten.rs$"],
  },
  {
    name: "flat-map-not-held",
    breaks:
      "a `flat_map()` a variable holds is the recognizers' JS iterator, sized `(0, None)` where std's counts",
    file: "src/lower/recognition/compiled_std.rs",
    find: '    ("FlatMap", "flat_map", false),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^std_flat_map.rs$"],
    snapshots: true,
  },
  {
    name: "take-not-held",
    breaks:
      "a held `take()` is the recognizers' JS iterator, not std's `Take`, which knows its `size_hint`",
    file: "src/lower/recognition/compiled_std.rs",
    find: '    ("Take", "take", true),\n',
    replace: "",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^held_skip_take.rs$"],
  },
  {
    name: "step-by-not-held",
    breaks:
      "a held `step_by()` is the recognizers' JS iterator, not std's `StepBy`",
    file: "src/lower/recognition/compiled_std.rs",
    find: '    ("StepBy", "step_by", false),\n',
    replace: "",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^adapters_own_nth_fold.rs$"],
  },
  {
    name: "chain-not-held",
    breaks:
      "a held `chain()` is the recognizers' JS iterator, not std's `Chain`",
    file: "src/lower/recognition/compiled_std.rs",
    find: '    ("Chain", "chain", false),\n',
    replace: "",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^adapter_drops.rs$"],
  },
  {
    name: "zip-sized-in-place",
    breaks:
      "a zip whose `size_hint()` is asked is made in place, `$lazyZip`, whose sides, JS's own, count nothing: `(0, Some(5))` of `v.iter().zip(v.iter().skip(2))` (#400)",
    file: "src/lower/recognition/compiled_std.rs",
    find: '    ("Zip", "zip", false),\n',
    replace: '    ("Zip", "zip", true),\n',
    tests: ["test/corpus.test.ts", "-t", "^zip_size_hints.rs$"],
  },
  {
    name: "io-chain-fn-not-compiled",
    breaks: "`core::io::chain`, which `Read::chain` calls and std keeps public but unstable, isn't compiled, so `@rust-js/std` can't be built (#1156)",
    file: "src/lower/recognition/compiled_std.rs",
    find: "const COMPILED_STD_IO_FNS: &[&str] = &[\"chain\", \"take\"];\n",
    replace: "const COMPILED_STD_IO_FNS: &[&str] = &[\"take\"];\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "question-mark-try-compiled",
    breaks: "a `?` of a `Result` in a compiled body, `Chain::read`'s, has std's `Try::branch` compiled for it, which the lowering writes itself, and `@rust-js/std` can't be built (#1156)",
    file: "src/lower/recognition/compiled_std.rs",
    find: "            if super::is_question_mark_written(tcx, callee, args) {\n",
    replace: "            if false && super::is_question_mark_written(tcx, callee, args) {\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "debug-shown-unnoted",
    breaks: "a type of std's adapter compiled as its meaning alone whose `{:?}` the crate asks isn't noted, so it stays the recognizers' and its `{:?}` is refused in the plain build (#1401)",
    file: "src/lower/recognition/compiled_std.rs",
    find: "                        types.insert(session_key(tcx, part));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^debug_of_adapters_compiled_as_their_meaning.rs$"],
  },
  {
    name: "debug-shown-type-not-compiled",
    breaks: "a type whose `{:?}` the crate asks isn't std's struct where it's held, its adapter's name alone answering, so its `{:?}` is refused (#1401)",
    file: "src/lower/recognition/compiled_std.rs",
    find: "                        .is_some_and(|(types, _)| types.contains(&session_key(tcx, ty)))\n",
    replace: "                        .is_some_and(|(types, _)| types.is_empty() && types.contains(&session_key(tcx, ty)))\n",
    tests: ["test/corpus.test.ts", "-t", "^debug_of_adapters_compiled_as_their_meaning.rs$"],
  },
  {
    name: "debug-shown-maker-of-any-type",
    breaks: "a maker of std's adapter compiled as its meaning alone, `filter`, is called compiled whatever type it makes, so a filter whose `{:?}` nothing asks is std's struct too, `iterator_filter(..)` where it was `range.filter(p)` (#1401)",
    file: "src/lower/recognition/compiled_std.rs",
    find: "        return is_compiled_std(tcx, id) && compiled_for(tcx, made);\n",
    replace: "        return is_compiled_std(tcx, id);\n",
    tests: ["test/corpus.test.ts", "-t", "^debug_of_adapters_compiled_as_their_meaning.rs$"],
    snapshots: true,
  },
];
