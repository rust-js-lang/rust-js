// Mutations of src/lower/library.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "library-check-every-crate-of-its-name",
    breaks:
      "each crate rustc loaded of a library's name is checked against its manifest, another release of it too, so a graph of cfg-if 0.1 and 1.0 is refused, its metadata \"isn't of the build its JS was made from\" (#1130)",
    file: "src/lower/library.rs",
    find: "            let loaded = (named.iter()).any(|&&krate| self.tcx.crate_hash(krate).to_string() == library.crate_hash);\n",
    replace: "            let loaded = (named.iter()).all(|&&krate| self.tcx.crate_hash(krate).to_string() == library.crate_hash);\n",
    tests: ["test/sysroot-names.test.ts", "-t", "two releases of one crate"],
  },
  {
    name: "library-checked-against-sysroot-crates",
    breaks:
      "a library is checked against std's own crate of its name, of rustc's sysroot, where rustc loaded that one alone: a program given a dependency's `cfg-if` it never reaches is refused, \"isn't of the build its JS was made from\", itertools' own tests (#1420)",
    file: "src/lower/library.rs",
    find: "                .filter(|&&krate| !super::recognition::from_sysroot(self.tcx, krate.as_def_id()))\n",
    replace: "",
    tests: ["test/sysroot-names.test.ts", "-t", "never reaches"],
  },
  {
    name: "library-type-rcs-counted-for-unseen-consumers",
    breaks:
      "a library generic over another library's type counts its `Rc`s for consumers it can't see, though that library counts none, so React's reconciler, a library, is refused for `react`'s context (ADR 0852, amended)",
    file: "src/lower/library.rs",
    find: "                !adt.did().is_local() && self.is_library(krate) && !self.counts_rc(krate, pointee)\n",
    replace:
      "                !adt.did().is_local() && self.is_library(krate) && !self.counts_rc(krate, pointee) && false\n",
    tests: ["test/crates.test.ts", "-t", "counts another library"],
  },
  {
    name: "library-crate-hash-unchecked",
    breaks:
      "a library's manifest is taken beside the metadata of another build of it",
    file: "src/lower/library.rs",
    find: "            if !named.is_empty() && !loaded {\n",
    replace: "            if false && !named.is_empty() && !loaded {\n",
    tests: ["test/crates.test.ts", "-t", "another build"],
  },
  {
    name: "library-drops-unlisted",
    breaks:
      "a library's manifest says none of its functions takes a drop, so a consumer compiles its own copy of `second`, which takes one, not calling the library's",
    file: "src/lower/library.rs",
    find: "        param(&mut params, index).marks.insert(ParamFact::Drop);\n",
    replace: "        param(&mut params, index);\n",
    tests: [
      "test/crates.test.ts",
      "-t",
      "a library's generic function that takes a drop is called with the consumer's",
    ],
  },
  {
    name: "library-facts-unsaid",
    breaks:
      "a library's manifest doesn't say what its functions ask of their type parameters",
    file: "src/lower/library.rs",
    find: "        param(&mut params, index).facts.push(fact.key().to_string());\n",
    replace: "        let _ = (param(&mut params, index), fact);\n",
    tests: ["test/crates.test.ts", "-t", "size of its consumer's type"],
  },
  {
    name: "library-location-unread",
    breaks:
      "a consumer reads no library function as taking its caller's location, and gives it none",
    file: "src/lower/library.rs",
    find: "        self.item(id)\n            .is_some_and(|item| item.calling.has(ItemFact::TakesLocation))\n",
    replace:
      "        self.item(id)\n            .is_some_and(|item| false && item.calling.has(ItemFact::TakesLocation))\n",
    tests: ["test/crates.test.ts", "-t", "two crates: track_caller"],
  },
  {
    name: "private-trait-impl-unexported",
    breaks:
      "a library's private type's impl its generic code asks a consumer for isn't exported",
    file: "src/lower/library.rs",
    find: "    matches!(tcx.def_kind(id), rustc_hir::def::DefKind::Impl { of_trait: true })\n        || id\n",
    replace: "    id\n",
    tests: ["test/crates.test.ts", "-t", "private type's impl"],
  },
  {
    name: "library-stepped-unsaid",
    breaks: "a library's manifest says no type parameter it steps",
    file: "src/lower/library.rs",
    find: "        param(&mut params, index).stepped.push(key(tcx, by));\n",
    replace: "        let _ = (param(&mut params, index), by);\n",
    tests: ["test/crates.test.ts", "-t", "two crates: extension_default"],
  },
  {
    name: "library-shape-any",
    breaks:
      "a library's contract read as counting every type of its `Rc`s, so its consumer counts its context, which it hands as the value itself (ADR 0852)",
    file: "src/lower/library.rs",
    find: "        (Some(of), Some((head, args))) => {\n            *of == head_key(tcx, &head)\n",
    replace: "        (Some(of), Some((head, args))) => {\n            true\n",
    tests: ["test/crates.test.ts", "-t", "two crates: rc_counted_by_its_library_alone$"],
  },
  {
    name: "instance-imported",
    breaks:
      "a library's function the crate compiles itself is still the library's item, imported where it's called, `import { createContext }`, so its own is never called (ADR 0914)",
    file: "src/lower/library.rs",
    find: "        if id.is_local() || self.is_instance(id) || super::recognition::not_compiled_for_build(self.tcx, id) {\n",
    replace:
      "        if id.is_local() || super::recognition::not_compiled_for_build(self.tcx, id) {\n",
    tests: [
      "test/crates-compiled-here.test.ts",
      "-t",
      "Copy bound takes a copy only where one is given",
    ],
  },
  {
    name: "library-copy-unlisted",
    breaks:
      "a library's contract says none of its functions takes a copy, so its consumer compiles its own `twice`, which its library's takes (ADR 0914)",
    file: "src/lower/library.rs",
    find: "        param(&mut params, index).marks.insert(ParamFact::Copy);\n",
    replace: "        let _ = (&mut params, index);\n",
    tests: [
      "test/crates-compiled-here.test.ts",
      "-t",
      "is its program's, alone or a dependency",
    ],
  },
  {
    name: "library-clone-unwritten",
    breaks:
      "a library's contract says no export takes a `TClone`, so its consumer gives `dup` none where it takes one (ADR 0914, amended)",
    file: "src/lower/library.rs",
    find: "        param(&mut params, index).marks.insert(ParamFact::Clone);\n",
    replace: "",
    tests: ["test/cross-crate-items.test.ts", "-t", "contract says it takes"],
  },
  {
    name: "cell-shape-dyn-anywhere",
    breaks:
      "a library's checked `RefCell<Vec<Rc<dyn Fn()>>>` is taken to be any `RefCell<Vec<Rc<_>>>`, so its consumer's `RefCell<Vec<Rc<Node>>>` is checked, `$borrowMut(siblings, true)` (#374, ADR 0960)",
    file: "src/lower/library.rs",
    find: "        _ if dyn_shape && unsized_here => true,\n",
    replace: "        _ if dyn_shape => true,\n",
    tests: [
      "test/crates.test.ts",
      "-t",
      "a library's checked cell of a Vec of dyn Fns",
    ],
  },
  {
    name: "library-unboxed-unwritten",
    breaks:
      "a library's contract says none of its type parameters is unboxed, so a consumer that gives `flush_sync` a `()` calls the export, whose `Some(())` is `undefined` (ADR 0051, amended)",
    file: "src/lower/library.rs",
    find: "        param(&mut params, index).marks.insert(ParamFact::Unboxed);\n",
    replace: "",
    tests: ["test/cross-crate-items.test.ts", "-t", "unboxed function"],
  },
  {
    name: "package-root-path-unread",
    breaks:
      "an item a module names through its library's root, `kids::bump`, is imported by its module's file, where its package's entry re-exports it, `import {Children} from 'react'` (ADR 1008, amended)",
    file: "src/lower/library.rs",
    find: "            && paths.rooted.contains(&(module, id))\n",
    replace: "            && false\n",
    tests: ["test/libraries.test.ts", "-t", "entry re-exports"],
  },
  {
    name: "package-root-path-of-the-crate",
    breaks:
      "an item one module names through its library's root is imported by the package's name in every module, one naming its module's path too, where each module's import follows its own `use` (the user's decision, ADR 1008, amended)",
    file: "src/lower/library.rs",
    find: ".contains(&(module, id))",
    replace: ".iter().any(|&(_, rooted)| rooted == id)",
    tests: ["test/libraries.test.ts", "-t", "entry re-exports"],
  },
  {
    name: "library-built-unread",
    breaks:
      "a consumer reads no library's structs it makes, taking the reconciler's fiber for JS's (rd221)",
    file: "src/lower/library.rs",
    find: "        (self.dependencies.libraries.values()).any(|library| library.built.contains(&key))\n",
    replace:
      "        (self.dependencies.libraries.values()).any(|library| library.built.contains(&key) && false)\n",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "a struct a library builds is the program's own data",
    ],
  },
  {
    name: "std-keyed-by-hash",
    breaks:
      "std's items are keyed by their `DefPathHash`, which a program's session, on the shipped std, finds none of in `@rust-js/std` built of a `core` built to unwind (#424)",
    file: "src/lower/library.rs",
    find: "    match super::recognition::is_std_crate(tcx, id) {",
    replace: "    match false && super::recognition::is_std_crate(tcx, id) {",
    tests: ["test/corpus.test.ts", "-t", "^std_peekable.rs$"],
  },
  {
    name: "reexported-names-unlisted",
    breaks:
      "a library's module's re-exported names aren't what its JS exports, so a namespace's method that's a function it re-exports is in no manifest, and a consumer's call is refused (rd292b)",
    file: "src/lower/library.rs",
    find: "            .chain(\n                module\n                    .reexports\n                    .iter()\n                    .flat_map(|reexport| reexport.named.iter().map(|(_, alias)| alias)),\n            )\n",
    replace: "",
    tests: ["test/namespaces.test.ts", "-t", "library's namespace's method"],
  },
  {
    name: "library-tracked-method-unrefused",
    breaks:
      "a library's trait method without a default whose entries take its caller's location is exported, which its manifest can't say",
    file: "src/lower/library.rs",
    find: "        .filter(|&&id| tcx.trait_of_assoc(id).is_some() && !fns.contains_key(&id))\n",
    replace:
      "        .filter(|&&id| false && tcx.trait_of_assoc(id).is_some() && !fns.contains_key(&id))\n",
    tests: [
      "test/crates.test.ts",
      "-t",
      "tracked trait method without a default",
    ],
  },
  {
    name: "with-this-arguments-unread-by-consumer",
    breaks: "a consumer reads no library's contract of the `function::with_this` functions that read `js::arguments()`, so its `dyn Fn` call of one leaves out a trailing `undefined` (#553)",
    file: "src/lower/library.rs",
    find: "            !library.builtin && (library.with_this_arguments.iter()).any(|shape| may_be_shape(self.tcx, shape, args))\n",
    replace: "            !library.builtin && false && (library.with_this_arguments.iter()).any(|shape| may_be_shape(self.tcx, shape, args))\n",
    tests: ["test/arguments.test.ts"],
  },
  {
    name: "with-this-arguments-any-shape",
    breaks: "a consumer's `dyn Fn` call of any parameters gives every argument where a library's `function::with_this` function of other parameters reads `js::arguments()`, `f(undefined)` of one of one parameter (#553)",
    file: "src/lower/library.rs",
    find: "            !library.builtin && (library.with_this_arguments.iter()).any(|shape| may_be_shape(self.tcx, shape, args))\n",
    replace: "            !library.builtin && !library.with_this_arguments.is_empty()\n",
    tests: ["test/arguments.test.ts"],
  },
  {
    name: "std-library-by-crate-name",
    breaks:
      "std's crates are known by their crate's key, not `@rust-js/std`'s, so the facts of std's compiled bodies, a writer's write that may fail, are never found (#455, ADR 1054, #1130)",
    file: "src/lower/library.rs",
    find: "        Some(_) => crate::runtime::PACKAGE.to_string(),\n",
    replace: "        Some(_) => crate::library::key(tcx.crate_name(krate).as_str(), &crate_id(tcx, krate)),\n",
    tests: ["test/corpus.test.ts", "-t", "writer"],
  },
  {
    name: "owned-by-return-of-a-library-unread",
    breaks:
      "a library's function whose return alone owns a callback is taken to keep it apart, so its consumer's callbacks count the library's request, which a library that counts none hands it: refused",
    file: "src/lower/library.rs",
    find: "            .is_some_and(|item| item.calling.owned_by_return.contains(&index))\n",
    replace: "            .is_some_and(|item| item.calling.owned_by_return.contains(&index) && false)\n",
    tests: ["test/crates.test.ts", "-t", "owns its callbacks alone"],
  },
  {
    name: "nulled-params-unread",
    breaks:
      "a consumer reads no library function's own nullable parameter as `null` at each call, so it gives it what may be `undefined` (rd613)",
    file: "src/lower/library.rs",
    find: "            .is_some_and(|item| item.calling.nulled_params.contains(&(index as u32)))\n",
    replace: "            .is_some_and(|_| false)\n",
    tests: ["test/cross-crate-items.test.ts", "-t", "nullable parameter its calls give null"],
  },
  {
    name: "through-path-unread",
    breaks:
      "a library's item named through its module that re-exports it is imported from its home all the same (rd704)",
    file: "src/lower/library.rs",
    find: "        Some(through.unwrap_or_else(|| (item.from.clone(), item.export.clone())))\n",
    replace: "        Some(through.filter(|_| false).unwrap_or_else(|| (item.from.clone(), item.export.clone())))\n",
    tests: ["test/reexports-through.test.ts", "-t", "a use through a library's module that re-exports its item imports it from there"],
  },
  {
    name: "library-dictionary-const-lacked",
    breaks: "a library's dictionaries, which carry every constant, are taken to lack those its code reads nowhere, and a consumer's generic read of one is refused (#597)",
    file: "src/lower/library.rs",
    find: "                && (library.dictionary_consts.as_ref()).is_some_and(|consts| !consts.contains(&konst_key));\n",
    replace: "                && (library.dictionary_consts.as_ref()).is_none_or(|consts| !consts.contains(&konst_key));\n",
    tests: ["test/crates.test.ts", "test/rust-js-test-programs.test.ts"],
  },
  {
    name: "program-dyn-trait-keys-unread",
    breaks: "a program's traits whose `dyn`s drop nothing are never found, so a test's `dyn` of one, of what drops, is never dropped (#597)",
    file: "src/lower/library.rs",
    find: "        library.dyns_drop_nothing.contains(&key(self.tcx, tr)).then_some(())?;\n",
    replace: "        None::<()>?;\n",
    tests: ["test/rust-js-test-programs.test.ts", "-t", "makes a dyn of the program's trait"],
  },
  {
    name: "built-template-unwritten",
    breaks:
      "a library's function asking the layout of a type built of its type parameters is refused, as its manifest can't say the type to its consumers (#737)",
    file: "src/lower/library.rs",
    find: "        let Some(of) = built_types::template(tcx, foreign, id, slot.ty) else {\n",
    replace: "        let Some(of) = built_types::template(tcx, foreign, id, slot.ty).filter(|_| false) else {\n",
    tests: ["test/crates.test.ts", "-t", "built_layout_of_a_library"],
  },
  {
    name: "partial-cmp-is-cmp-unread",
    breaks: "a consumer reads no library impl's `partial_cmp` as `Some(self.cmp(other))`, whatever its contract lists (#964)",
    file: "src/lower/library.rs",
    find: "                .is_some_and(|library| library.partial_cmp_is_cmp.contains(&key(self.tcx, imp)))\n",
    replace: "                .is_some_and(|library| library.partial_cmp_is_cmp.is_empty())\n",
    tests: ["test/crates.test.ts", "-t", "derived_partial_cmp_is_cmp"],
  },
  {
    name: "fails-of-an-instance-unlisted",
    breaks:
      "whether a library's impl's method may fail asks `item`, which has none of an instance, so a writer's own `write_str` the program compiles itself throws its `fmt::Error` where it's an `Err` (#1023)",
    file: "src/lower/library.rs",
    find: "        let item = listed(id).or_else(|| listed(self.tcx.trait_impl_of_assoc(id)?));\n",
    replace: "        let item = self.item(id).or_else(|| self.item(self.tcx.trait_impl_of_assoc(id)?));\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_own_methods_through_its_dictionary"],
  },
  {
    name: "library-recursive-clone-inline",
    breaks:
      "an item rust-js compiled is the crate's own alone, not a library's too, so a clone of a library's type inside itself isn't a function that calls itself, and never ends",
    file: "src/lower/library.rs",
    find: "        id.is_local() || self.in_library(id)\n",
    replace: "        id.is_local()\n",
    tests: ["test/crates.test.ts", "-t", "two crates: tree"],
  },
  {
    name: "library-writes-never-fail",
    breaks:
      "a consumer doesn't read whether a library's writes may fail, so std's `write_all` of the library's `io::Write`, whose `write` fails, is taken as never failing and its `Err` thrown past `put(..).is_err()` (#1072)",
    file: "src/lower/library.rs",
    find: ".any(|library| library.writes_may_fail)",
    replace: ".any(|_| false)",
    tests: ["test/crates.test.ts", "-t", "two crates: library_io_writer_fails"],
  },
  {
    name: "library-closure-clone-unasked",
    breaks: "a consumer asks no library's contract of its closures, so it decides of each itself, and its `$clone(f)` of a plain function throws (#1202)",
    file: "src/lower/library.rs",
    find: "        (self.library(id.krate)?.cloned_closures.get(&key(self.tcx, id))).copied()\n",
    replace: "        (self.library(id.krate)?.cloned_closures.get(&key(self.tcx, id))).and(None)\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_closures_cloned_as_made"],
  },
  {
    name: "uncounted-library-rcs-unseen",
    breaks:
      "a crate counting an `Rc` a library that counts none of hands it isn't refused, and the two read it apart (ADRs 0844, 0852)",
    file: "src/lower/library.rs",
    find: "                false => reached\n                    .uncounted_rcs\n                    .push((pointee, tcx.crate_name(of.krate).to_string())),\n",
    replace: "                false => {}\n",
    tests: ["test/crates.test.ts", "-t", "reads no count"],
  },
  {
    name: "library-rc-unreached",
    breaks:
      "an `Rc` a library hands a crate isn't counted, though the library counts it",
    file: "src/lower/library.rs",
    find: "                true => reached.counted_rcs.push(pointee),\n",
    replace: "                true => {}\n",
    tests: ["test/crates.test.ts", "-t", "two crates: rc_counted_by_its_library_alone$"],
  },
  {
    name: "instance-writers-take-nothing",
    breaks:
      "the `{:#?}` of a library's generic function the consumer compiles itself isn't seen, so its writers don't take whether to be pretty, and it shows `Loud { n: 2 }` on one line (#1325)",
    file: "src/lower/library.rs",
    find: "            .is_some_and(|library| !library.builtin && library.writers_take_options)\n",
    replace: "            .is_some_and(|library| !library.builtin && library.writers_take_options && false)\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_instances_ask_what_the_crate_doesnt"],
  },
  {
    name: "shown-params-unexported",
    breaks:
      "a library's manifest marks no type parameter its functions show pretty or in hex, so its consumer's `Plain` shows `Plain { n: 255 }` of its `{:x?}` (#1331)",
    file: "src/lower/library.rs",
    find: "        if shown.pretty {\n            marks.insert(ParamFact::ShownPretty);\n        }\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: library_shows_a_consumers_value_with_options"],
  },
  {
    name: "std-asks-consumers-keep-capacity",
    breaks:
      "`@rust-js/std`'s manifest's `keeps_capacity` makes every consumer keep each `Vec`'s capacity, though std asks none of what they give it: right, but `$grew` beside every push of every program (#1339)",
    file: "src/lower/library.rs",
    find: "        (self.dependencies.libraries.values()).any(|library| !library.builtin && library.keeps_capacity)\n",
    replace: "        (self.dependencies.libraries.values()).any(|library| library.keeps_capacity)\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_extend_indexed\\.rs$"],
    snapshots: true,
  },
  {
    name: "export-rc-counted-unmarked",
    breaks: "a library's export marks no type parameter whose `Rc` its JS counts, so a consumer that counts none of its `Bar`'s reads the library's counted JS as the value (#1234)",
    file: "src/lower/library.rs",
    find: "            param(&mut params, index).marks.insert(ParamFact::Counted);\n",
    replace: "",
    tests: [
      "test/crates.test.ts",
      "-t",
      "^two crates: (rc_counted_by_its_librarys_call)$"
    ]
  },
  {
    name: "generic-pointee-handed-by-library",
    breaks: "a library's generic function's `Rc` of its type parameter is one it hands its consumer, so a consumer that counts what it gives it is refused, though it compiles its own (#1234)",
    file: "src/lower/library.rs",
    find: "            if decided_by_call && rustc_middle::ty::TypeVisitableExt::has_non_region_param(&pointee) {\n",
    replace: "            if false && decided_by_call && rustc_middle::ty::TypeVisitableExt::has_non_region_param(&pointee) {\n",
    tests: [
      "test/crates.test.ts",
      "-t",
      "^two crates: (rc_counted_by_a_consumers_call)$"
    ]
  },
  {
    name: "library-written-into-marks-left-out",
    breaks: "a library's function's type parameter it writes into a writer through its dictionary isn't marked `written_into`, so a consumer gives it a dictionary with no `fmtInto` (#1652)",
    file: "src/lower/library.rs",
    find: "        param(&mut params, index).marks.insert(ParamFact::WrittenInto);\n",
    replace: "",
    tests: ["test/writer-sink.test.ts", "-t", "library"],
  },
];
