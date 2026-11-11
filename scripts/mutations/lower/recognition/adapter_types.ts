// Mutations of src/lower/recognition/adapter_types.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "filter-not-back-stepped",
    breaks:
      "a filter of what knows its back isn't seen to know its own, and generic code's is refused from its back",
    file: "src/lower/recognition/adapter_types.rs",
    find: '                "std::iter::Map"\n                    | "std::iter::Filter"\n                    | "std::iter::FilterMap"\n',
    replace:
      '                "std::iter::Map"\n                    | "std::iter::FilterMap"\n',
    tests: ["test/corpus.test.ts", "-t", "^filter_ends.rs$"],
  },
  {
    name: "filter-map-not-back-stepped",
    breaks: "a `filter_map` of what knows its back isn't seen to know its own",
    file: "src/lower/recognition/adapter_types.rs",
    find: '                "std::iter::Map"\n                    | "std::iter::Filter"\n                    | "std::iter::FilterMap"\n',
    replace:
      '                "std::iter::Map"\n                    | "std::iter::Filter"\n',
    tests: ["test/corpus.test.ts", "-t", "^(filter_ends|held_filter_map_from_its_back).rs$"],
  },
  {
    name: "map-back-unknown",
    breaks:
      "a map of an iterator that knows its back doesn't, so `rfind` of a generic one's map is refused",
    file: "src/lower/recognition/adapter_types.rs",
    find: '                "std::iter::Map"\n                    | "std::iter::Filter"\n                    | "std::iter::FilterMap"\n',
    replace:
      '                "std::iter::Filter"\n                    | "std::iter::FilterMap"\n',
    tests: ["test/corpus.test.ts", "-t", "^(back_steps|std_rev|held_enumerate).rs$"],
  },
  {
    name: "enumerate-length-unknown",
    breaks:
      "an `enumerate` of what knows its length isn't seen to know its own",
    file: "src/lower/recognition/adapter_types.rs",
    find: "                \"std::iter::Map\"\n                    | \"std::iter::Inspect\"\n                    | \"std::iter::Copied\"\n                    | \"std::iter::Cloned\"\n                    | \"std::iter::Enumerate\"\n                    | \"std::iter::Rev\"\n",
    replace: "                \"std::iter::Map\"\n                    | \"std::iter::Inspect\"\n                    | \"std::iter::Copied\"\n                    | \"std::iter::Cloned\"\n                    | \"std::iter::Rev\"\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(fuse_enumerate_ends|enumerate_of_known_length).rs$",
    ],
  },
  {
    name: "map-length-unknown",
    breaks:
      "a map of an iterator that knows its length doesn't, so its `len()` is refused",
    file: "src/lower/recognition/adapter_types.rs",
    find: "                \"std::iter::Map\"\n                    | \"std::iter::Inspect\"\n                    | \"std::iter::Copied\"\n                    | \"std::iter::Cloned\"\n                    | \"std::iter::Enumerate\"\n                    | \"std::iter::Rev\"\n",
    replace: "                \"std::iter::Copied\"\n                    | \"std::iter::Cloned\"\n                    | \"std::iter::Enumerate\"\n                    | \"std::iter::Rev\"\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(exact_size_len|map_stepped_from_its_back|owned_maps_asked_their_length)\\.rs$",
    ],
  },
  {
    name: "rev-length-unknown",
    breaks: "a `rev()` of what knows its length isn't seen to know its own",
    file: "src/lower/recognition/adapter_types.rs",
    find: "                \"std::iter::Map\"\n                    | \"std::iter::Inspect\"\n                    | \"std::iter::Copied\"\n                    | \"std::iter::Cloned\"\n                    | \"std::iter::Enumerate\"\n                    | \"std::iter::Rev\"\n",
    replace: "                \"std::iter::Map\"\n                    | \"std::iter::Inspect\"\n                    | \"std::iter::Copied\"\n                    | \"std::iter::Cloned\"\n                    | \"std::iter::Enumerate\"\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(std_iterator_clones|len_made_in_place|owned_maps_asked_their_length)\\.rs$",
    ],
  },
  {
    name: "compiled-iterator-stepped-by-library",
    breaks:
      "`Iterator`'s methods of a compiled std iterator, a `Peekable`'s `next`, resolve to the library's compiled impl and are called there, not stepped as the JS iterator its struct is",
    file: "src/lower/recognition/adapter_types.rs",
    find: "            .is_some_and(|self_ty| is_compiled_std_iterator(tcx, self_ty))\n        && !super::compiled_instance(tcx, def_id, args)\n",
    replace:
      "            .is_some_and(|self_ty| is_compiled_std_iterator(tcx, self_ty))\n        && false\n        && !super::compiled_instance(tcx, def_id, args)\n",
    tests: ["test/corpus.test.ts", "-t", "^std_peekable.rs$"],
    snapshots: true,
  },
  {
    name: "compiled-provided-of-compiled-iterator-not-crates",
    breaks:
      "a compiled provided method of a compiled std iterator, `peekable()` of a `Fuse`, doesn't resolve to the library, and is refused",
    file: "src/lower/recognition/adapter_types.rs",
    find: "        && !super::compiled_instance(tcx, def_id, args)\n}\n",
    replace: "}\n",
    tests: ["test/corpus.test.ts", "-t", "^held_peekable.rs$"],
  },
  {
    name: "chain-not-cloned-apart",
    breaks: "cloning a `chain` of what's cloned is refused",
    file: "src/lower/recognition/adapter_types.rs",
    find: '        "std::iter::Chain" => Parted::Chain,\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^cloned_chains_and_cycles.rs$"],
    snapshots: true,
  },
  {
    name: "held-compiled-no-adapter",
    breaks:
      "a `map()` made in place, which a `rev()` steps from its back, isn't std's adapter, so it isn't made knowing its back",
    file: "src/lower/recognition/adapter_types.rs",
    find: "        !is_compiled_std_type(tcx, kind) || super::is_held_compiled(tcx, kind)\n",
    replace: "        !is_compiled_std_type(tcx, kind)\n",
    snapshots: true,
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(rev_of_closed_items|rev_lazy_drops|std_rev|try_folds_of_stages_unwind).rs$",
    ],
  },
  {
    name: "adapted-iterators-of-any-unknown",
    breaks:
      "a type parameter a std adapter holds that isn't one it steps, a `map(f)`'s `F: Fn(&T) -> U` or a `successors()`' `T`, is taken for an iterator whose hint may be the crate's, so a public `applied<T, U, F>` takes `U`'s size, which a JS caller can't give, and isn't declared (#796)",
    file: "src/lower/recognition/adapter_types.rs",
    find: "    let held: Vec<_> = (adapted_positions(tcx, adt.did()).into_iter())\n        .filter_map(|at| args.get(at)?.as_type())\n",
    replace:
      "    let held: Vec<_> = (args.types())\n        .filter(|&held| !matches!(held.kind(), ty::Closure(..) | ty::FnDef(..) | ty::FnPtr(..)))\n",
    tests: ["test/declarations-impl-fn.test.ts", "-t", "a type parameter bound by Fn is declared as its function"],
  },
  {
    name: "cloned-not-back-stepped",
    breaks: "a `cloned()` of a slice's items whose `Clone` runs code doesn't step from its back where its source does, and a zip of two is refused (#792)",
    file: "src/lower/recognition/adapter_types.rs",
    find: "                    | \"std::iter::Copied\"\n                    | \"std::iter::Cloned\"\n                    | \"std::iter::Inspect\"\n",
    replace: "                    | \"std::iter::Copied\"\n                    | \"std::iter::Inspect\"\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_of_cloned_items_whose_clone_runs.rs$"],
  },
  {
    name: "cloned-length-not-kept",
    breaks: "a `cloned()` of a slice's items whose `Clone` runs code doesn't know its length where its source does, and a zip of two is refused (#792)",
    file: "src/lower/recognition/adapter_types.rs",
    find: "                    | \"std::iter::Copied\"\n                    | \"std::iter::Cloned\"\n                    | \"std::iter::Enumerate\"\n",
    replace: "                    | \"std::iter::Copied\"\n                    | \"std::iter::Enumerate\"\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_of_cloned_items_whose_clone_runs.rs$"],
  },
  {
    name: "decode-utf16-unadapted",
    breaks: "`DecodeUtf16<I>` isn't seen to adapt its `I`, so its `size_hint()` is refused (#1151)",
    file: "src/lower/recognition/adapter_types.rs",
    find: "    if !std_path(tcx, adt.did()).starts_with(\"std::iter::\") && !is_decode_utf16(tcx, ty) {\n",
    replace: "    if !std_path(tcx, adt.did()).starts_with(\"std::iter::\") {\n",
    tests: ["test/corpus.test.ts", "-t", "^decode_utf16_steps_as_asked.rs$"],
  },
  {
    name: "source-hint-of-successors-unkept",
    breaks: "a `successors` or a `repeat_with` an adapter holds is taken for one whose hint is unknown, so `successors(..).peekable().size_hint()` is refused (#1357)",
    file: "src/lower/recognition/adapter_types.rs",
    find: "        \"std::iter::Repeat\" | \"std::iter::RepeatN\" | \"std::iter::RepeatWith\" | \"std::iter::Successors\"\n",
    replace: "        \"std::iter::Repeat\" | \"std::iter::RepeatN\"\n",
    tests: ["test/corpus.test.ts", "-t", "^repeat_n_hint_held.rs$"],
  },
  {
    name: "once-with-hint-unkept",
    breaks: "`once_with(f)`'s `$onceWith` isn't said to give std's hint, so a `peekable()` of it is refused, where std's is `(1, Some(1))` (#1375)",
    file: "src/lower/recognition/adapter_types.rs",
    find: " | \"std::iter::OnceWith\"))",
    replace: "))",
    tests: ["test/corpus.test.ts", "-t", "^once_and_once_with_hint.rs$"],
  },
  {
    name: "inspect-length-unkept",
    breaks: "an `inspect` made knowing its back, `$mapEnds`, isn't taken for one that knows its source's length, so a zip of it a capacity-reading crate collects is refused (#1394)",
    file: "src/lower/recognition/adapter_types.rs",
    find: "                \"std::iter::Map\"\n                    | \"std::iter::Inspect\"\n                    | \"std::iter::Copied\"\n",
    replace: "                \"std::iter::Map\"\n                    | \"std::iter::Copied\"\n",
    tests: ["test/corpus.test.ts", "-t", "^zip_of_an_inspect_counted_where_capacity_is_read.rs$"],
  },
  {
    name: "back-as-front-of-a-rev-unknown",
    breaks: "a `rev()` of a `repeat_n` that MIR holds in a temporary isn't known to have a length, so its `rposition` is refused (#1139)",
    file: "src/lower/recognition/adapter_types.rs",
    find: "        || reversed_source(tcx, ty).is_some_and(|source| steps_back_as_front(tcx, source.peel_refs()))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "test/iterator-forms.test.ts", "-t", "^repeats_stepped_from_their_backs.rs$|^repeats_stepped_from_their_backs with"],
  },
];
