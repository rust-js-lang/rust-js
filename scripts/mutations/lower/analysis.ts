// Mutations of src/lower/analysis.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "type-constants-unexported",
    breaks: "a `pub` type's `pub` constants are no property of its object, so JS can't name `ReactiveFlags.RecursedCheck` (#498)",
    file: "src/lower/analysis.rs",
    find: "                    || exported_value_const(tcx, d.to_def_id())\n",
    replace: "",
    tests: ["test/impl-constants.test.ts"],
  },
  {
    name: "self-handle-counted-lowered",
    breaks:
      "a closure's handle on itself whose `Rc` the program counts, `Rc::strong_count`, is written as no JS too, its upgrade its own name, where its counts must be kept",
    file: "src/lower/analysis.rs",
    find: "    self_handles.uncounted(|pointee| counted.counts(tcx, pointee));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^self_handle_counted.rs$"],
  },
  {
    name: "types-only-module-unlisted",
    breaks:
      "a module of only `pub type`s and structs writes no `.d.ts`, so TypeScript importing its types finds no module (ADR 1005)",
    file: "src/lower/analysis.rs",
    find: "        if types && seen_modules.insert(module) {\n",
    replace: "        if false && types && seen_modules.insert(module) {\n",
    tests: ["test/cargo-in-source.test.ts", "-t", "declarations in source"],
  },

  {
    name: "untagged-counted-rc-unrefused",
    breaks:
      "an untagged enum's variant of an `Rc` the crate counts, `{ value, strong, weak }`, is tested `Array.isArray`, which it never is (ADR 0214, amended)",
    file: "src/lower/analysis.rs",
    find: "        if let Some(pointee) = super::recognition::rc_pointee(tcx, held)\n            && counted.counts(tcx, pointee)\n",
    replace:
      "        if let Some(pointee) = super::recognition::rc_pointee(tcx, held)\n            && counted.counts(tcx, pointee) && false\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "variant of an Rc the crate counts",
    ],
  },
  {
    name: "initializer-imports-unnamed",
    breaks:
      "a static of a module's binding, `static NOTIFY: fn() -> f64 = toast`, names no import, and lowering it panics",
    file: "src/lower/analysis.rs",
    find: "all_bodies.iter().copied().chain(initializers).collect();",
    replace: "all_bodies.to_vec();",
    tests: [
      "test/lowering-bindings.test.ts",
      "-t",
      "a static of a module's binding imports it",
    ],
  },
  {
    name: "numbered-variants-unnamed",
    breaks:
      "a numbered enum's variants are no module's `const`s, each read as its number, `payload._status = 0`, not `Pending`",
    file: "src/lower/analysis.rs",
    find: "        .filter(|&d| tcx.def_kind(d) == DefKind::Enum && bindings::numbered(tcx, tcx.adt_def(d)))\n",
    replace:
      "        .filter(|&d| tcx.def_kind(d) == DefKind::Enum && bindings::numbered(tcx, tcx.adt_def(d)) && false)\n",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "a library's repr enum's variant is imported by its name",
    ],
  },
  {
    name: "const-reexport-dropped",
    breaks:
      "`pub use priorities::IDLE` re-exports nothing, so `IDLE` isn't the module's",
    file: "src/lower/analysis.rs",
    find: "DefKind::Fn | DefKind::Const, _)) => true,",
    replace: "DefKind::Fn, _)) => true,",
    tests: ["test/modules.test.ts", "-t", "another name of the module's own"],
  },
  {
    name: "thread-local-storage-static",
    breaks:
      "std's storage for a `thread_local!` is taken as a static of the crate's, and rejected",
    file: "src/lower/analysis.rs",
    find: "                !tcx.is_foreign_item(d) && in_thread_local(tcx, d).is_none() && !markers.iter().any(|&(m, _)| m == d)\n",
    replace: "                !tcx.is_foreign_item(d) && !markers.iter().any(|&(m, _)| m == d)\n",
    tests: ["test/corpus.test.ts", "-t", "thread_local_syntax"],
  },
  {
    name: "js-import-const-written",
    breaks: "`js::import!`'s `const _` is written to the JS as a constant",
    file: "src/lower/analysis.rs",
    find: " && !bindings::is_mark(tcx, d.to_def_id())\n",
    replace: "\n",
    tests: ["test/modules.test.ts", "-t", "js::import"],
  },
  {
    name: "reexport-only-module-unwritten",
    breaks:
      "a module of only `pub use`s, Sidebar/index, gets no file, and what imports it finds nothing",
    file: "src/lower/analysis.rs",
    find: "if (reexports || defaulted()) && seen_modules.insert(module)",
    replace:
      "if (false && reexports || defaulted()) && seen_modules.insert(module)",
    tests: ["test/modules.test.ts", "-t", "only pub uses"],
  },
  {
    name: "const-thread-local-init-unseen",
    breaks:
      "a `const { .. }` thread-local's value is a `const` of std's name, `__RUST_STD_INTERNAL_INIT`, which nothing reads",
    file: "src/lower/analysis.rs",
    find: "                .filter(|&d| matches!(tcx.def_kind(d), DefKind::Const)),\n",
    replace:
      "                .filter(|&d| matches!(tcx.def_kind(d), DefKind::Const) && false),\n",
    tests: [
      "test/modules.test.ts",
      "-t",
      "only read and set is the module's let",
    ],
  },
  {
    name: "default-only-module-fileless",
    breaks: "a module of only another module's default gets no file",
    file: "src/lower/analysis.rs",
    find: "        let defaulted = || !super::default_exports::default_exports(tcx, module).is_empty();\n",
    replace: "        let defaulted = || false;\n",
    tests: ["test/modules.test.ts", "-t", "default of another module"],
  },
  {
    name: "coded-inline-consts-none",
    breaks: "a `const { .. }` block of no JS value is refused",
    file: "src/lower/analysis.rs",
    find: "        .chain(coded_inline_consts(tcx, all_bodies))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "slice_uninit"],
    snapshots: true,
  },
  {
    name: "mir-quiet-locals-none",
    breaks:
      "MIR knows no quiet thread-local, each borrowed through `$withBorrow`",
    file: "src/lower/analysis.rs",
    find: "    let quiet_locals = plain_locals::plain_thread_locals(\n        tcx,\n        all_bodies,\n        (thread_local_inits.values().copied()).filter(|key| !lazy_locals.contains(key)),\n        &quiet,\n    );\n",
    replace: "    let quiet_locals: IndexSet<_> = IndexSet::default();\n",
    tests: ["test/modules.test.ts", "-t", "thread-local only read and set"],
  },
  {
    name: "analysis-root-always-a-file",
    breaks:
      "a crate root of `mod`s alone, an app's list of its files, writes an empty JS file of its own, `src/lib.js`",
    file: "src/lower/analysis.rs",
    find: "    let mut modules = Vec::new();\n    let mut seen_modules = IndexSet::default();\n",
    replace:
      "    let mut modules = vec![rustc_span::def_id::CRATE_MOD_ID];\n    let mut seen_modules = IndexSet::from_iter([rustc_span::def_id::CRATE_MOD_ID]);\n",
    tests: ["test/modules.test.ts", "-t", "a root of modules alone"],
  },
  {
    name: "analysis-import-alone-no-file",
    breaks:
      "a module whose only item is a `js::import!` gets no file, and what it imports is lost",
    file: "src/lower/analysis.rs",
    find: '        if bindings::marks(tcx, module, "import").next().is_some() && seen_modules.insert(module) {',
    replace: "        if false && seen_modules.insert(module) {",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a root of modules alone|js::import! imports a module",
    ],
  },
  {
    name: "js-object-deref-dictionary",
    breaks:
      "a JS object's `Deref` to the class it extends is given a dictionary, whose `deref` is never lowered",
    file: "src/lower/analysis.rs",
    find: "        .filter(|&id| !is_js_object_deref(tcx, id))\n",
    replace: "",
    tests: [
      "test/compiler.test.ts",
      "-t",
      "an untagged enum is its payload, told apart",
    ],
  },
  {
    name: "static-reexport-fileless",
    breaks:
      "a module of only a `pub use` of a binding's static has no file, and its re-export is lost",
    file: "src/lower/analysis.rs",
    find: "            Some(rustc_hir::def::Res::Def(DefKind::Static { .. }, id)) => is_binding(tcx, id),\n",
    replace:
      "            Some(rustc_hir::def::Res::Def(DefKind::Static { .. }, _)) => false,\n",
    tests: ["test/modules.test.ts", "-t", "JS module's binding is re-exported"],
  },
  {
    name: "reexport-private-use",
    breaks:
      "a private `use` of another module's function is re-exported, though Rust exports it nowhere",
    file: "src/lower/analysis.rs",
    find: "    tcx.visibility(leaf.def_id).is_public()\n        && (match path.res.value_ns {\n",
    replace: "    true\n        && (match path.res.value_ns {\n",
    tests: [
      "test/modules.test.ts",
      "-t",
      "a use that renames another module's function imports it as that",
    ],
  },
  {
    name: "drop-runs-asked-crate-wide",
    breaks:
      "a type a caller gives on, naming its const parameter, `Buffer<SIZE>`, is asked about where that parameter isn't, and rustc crashes",
    file: "src/lower/analysis.rs",
    find: "        true => ty::TypingEnv::non_body_analysis(tcx, owner),\n",
    replace: "        true => ty::TypingEnv::fully_monomorphized(),\n",
    tests: ["test/corpus.test.ts", "-t", "^const_param_drops.rs$"],
  },
  {
    name: "hand-written-serde-as-derived",
    breaks:
      "a hand-written serde impl is written as a derived codec too, two functions of one name",
    file: "src/lower/analysis.rs",
    find: "        .filter(|&&id| serde_impl(tcx, id).is_some() && serde_derived(tcx, id))\n",
    replace: "        .filter(|&&id| serde_impl(tcx, id).is_some())\n",
    tests: ["test/serde.test.ts", "-t", "hand-written Deserialize"],
  },
  {
    name: "consumer-rcs-uncounted",
    breaks:
      "a crate using a library counts only the `Rc`s it reads, though the library counts ones it shares (ADR 0852): its clone of one its library made and counts isn't counted, and the library's `strong_count` reads `1` where Rust's is `2`",
    file: "src/lower/analysis.rs",
    find: "                (params_may_drop, &reached.counted_rcs, &|owner, index, pointee| {\n",
    replace:
      "                (params_may_drop, &[], &|owner, index, pointee| {\n",
    tests: [
      "test/crates.test.ts",
      "-t",
      "two crates: rc_per_type|two crates: rc_counted_by_its_library_alone|three crates: rc_generic",
    ],
  },
  {
    name: "library-rcs-uncounted",
    breaks:
      "a library's type parameters are taken to have no destructor, so it counts no generic `Rc` of what its consumers' may drop, and drops nothing of an associated type its consumers' may give one (ADRs 0844, 0852)",
    file: "src/lower/analysis.rs",
    find: "    let params_may_drop = crate::lower::traits::may_have_destructors(tcx, (&foreign, library));\n    let reached = foreign.reached(all_bodies);\n",
    replace:
      "    let params_may_drop = crate::lower::traits::may_have_destructors(tcx, (&foreign, false));\n    let reached = foreign.reached(all_bodies);\n",
    tests: [
      "test/crates.test.ts",
      "-t",
      "reads no count|two crates: assoc_drop",
    ],
  },
  {
    name: "fn-given-always-quiet",
    breaks:
      "a function given to `with_borrow` that borrows its own key mutably is taken as borrowing nothing, so its `with_borrow_mut` doesn't throw where Rust panics",
    file: "src/lower/analysis.rs",
    find: "            plain_locals::Given::Fn(f, args) => !cell_borrows.call_asks(\n",
    replace:
      "            plain_locals::Given::Fn(f, args) => true || !cell_borrows.call_asks(\n",
    tests: ["test/lowering-cells.test.ts", "-t", "given a function to run"],
  },
  {
    name: "library-default-named-after-file",
    breaks:
      "a library's module's default is imported named after the module's file, `import version`, not its item, `import VERSION`",
    file: "src/lower/analysis.rs",
    find: "        uses.named.insert(export, imported.name.clone());\n",
    replace: "",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "default export is imported as its default",
    ],
  },
  {
    name: "fn-dyn-library-params-ungiven",
    breaks:
      "a library's closure holding its type parameter's function, made a `dyn Fn`, drops only what its own calls give, though a consumer may give the parameter a destructor (#580)",
    file: "src/lower/analysis.rs",
    find: "        (params_may_drop, (!library).then_some(&given_a_drop), reached.functions),\n",
    replace: "        (params_may_drop, Some(&given_a_drop), reached.functions),\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_of_a_params_function.rs$"],
  },
  {
    name: "any-drops-of-any-library",
    breaks:
      "a crate whose library has destructors but makes no `dyn Any` of one drops what its `catch_unwind` caught, `error.impl.$drop?.(error.value)`, as React's react crate did of shared's guards",
    file: "src/lower/analysis.rs",
    find: "(params_may_drop, foreign.makes_dropping_anys())",
    replace: "(params_may_drop, foreign.any())",
    tests: ["test/cross-crate-items.test.ts", "-t", "guards make no dyn Any"],
  },
  {
    name: "class-display-dictionary-emitted",
    breaks:
      "a class's `impl Display` is emitted as a dictionary too, `queenPositionDisplay`, besides its `toString` (#358)",
    file: "src/lower/analysis.rs",
    find: "        .filter(|&id| super::classes::displayed_class(tcx, id).is_none())\n",
    replace: "",
    tests: ["test/classes.test.ts", "-t", "Display is its prototype"],
  },
  {
    name: "library-fn-dyns-always-drop",
    breaks:
      "a `dyn Fn` a library hands the crate drops though no library makes one of what drops, so the crate's own `Rc<dyn Fn()>` listeners are `{ value, strong, weak }` (ADR 0626, amended)",
    file: "src/lower/analysis.rs",
    find: "    let library_drops = (reached.fn_dyns_of.iter()).any(|name| foreign.drops_fn_dyns(name));\n",
    replace: "    let library_drops = !reached.fn_dyns_of.is_empty();\n",
    tests: [
      "test/cross-crate-dyn-fns.test.ts",
      "-t",
      "a library's dyn Fn drops in its consumer only where",
    ],
  },
  {
    name: "library-fn-dyns-never-drop",
    breaks:
      "a `dyn Fn` a library makes of a closure holding a value with a destructor, handed to the crate, is dropped as nothing (ADR 0626, amended)",
    file: "src/lower/analysis.rs",
    find: "    let library_drops = (reached.fn_dyns_of.iter()).any(|name| foreign.drops_fn_dyns(name));\n",
    replace:
      "    let library_drops = false && (reached.fn_dyns_of.iter()).any(|name| foreign.drops_fn_dyns(name));\n",
    tests: [
      "test/cross-crate-dyn-fns.test.ts",
      "-t",
      "a library's dyn Fn of what drops is dropped",
    ],
  },
  {
    name: "dropping-dyn-fn-given-unrefused",
    breaks:
      "a crate whose `dyn Fn`s may hold what drops isn't refused using a library whose `dyn Fn`s drop nothing, which drops nothing of the one it's given (ADR 0626, amended)",
    file: "src/lower/analysis.rs",
    find: "    if drops && let Some(name) = (reached.fn_dyns_of.iter()).find(|name| !foreign.drops_fn_dyns(name)) {\n",
    replace:
      "    if drops && let Some(name) = (reached.fn_dyns_of.iter()).find(|name| !foreign.drops_fn_dyns(name) && false) {\n",
    tests: [
      "test/cross-crate-dyn-fns.test.ts",
      "-t",
      "and refuses one it would drop nothing of",
    ],
  },
  {
    name: "dropping-dyn-fn-unnamed",
    breaks:
      "a crate refused for a `dyn Fn` it makes that holds what drops isn't told which, nor what it holds (rd553, ADR 0626, amended)",
    file: "src/lower/analysis.rs",
    find: "        closures::refuse_dropping_fn_dyn(tcx, name, made.first().filter(|_| !library_drops), &self_handles);\n",
    replace: "        closures::refuse_dropping_fn_dyn(tcx, name, made.first().filter(|_| false), &self_handles);\n",
    tests: ["test/crates.test.ts", "-t", "callback is a public field"],
  },
  {
    name: "dropping-dyn-fn-library-refused",
    breaks:
      "a crate whose `dyn Fn`s may hold what drops is refused using a library whose `dyn Fn`s drop too, so one that drops what it's given is refused (ADR 0626, amended)",
    file: "src/lower/analysis.rs",
    find: "    if drops && let Some(name) = (reached.fn_dyns_of.iter()).find(|name| !foreign.drops_fn_dyns(name)) {\n",
    replace:
      "    if drops && let Some(name) = (reached.fn_dyns_of.iter()).next() {\n",
    tests: [
      "test/cross-crate-dyn-fns.test.ts",
      "-t",
      "and refuses one it would drop nothing of",
    ],
  },
  {
    name: "library-rcs-of-own-calls-unsettled",
    breaks:
      "a library takes no drop its own calls give a type parameter, so where it reads another library's context of a `Loud`, which that library counts none of, it's not refused and its last clone drops nothing (ADR 0852, amended)",
    file: "src/lower/analysis.rs",
    find: "            let runs = |ty, owner| runs_where(closure_facts, &counted, ty, owner);\n",
    replace:
      "            if library {\n                return (counted, given);\n            }\n            let runs = |ty, owner| runs_where(closure_facts, &counted, ty, owner);\n",
    tests: ["test/crates.test.ts", "-t", "counts another library"],
  },
  {
    name: "rc-drop-params-unsettled",
    breaks:
      "the `Rc`s counted are taken before any type parameter is given a drop, so a generic `Rc<T>` whose instances no caller shows, of an impl's own parameter a dictionary gives, a `dyn Keep`'s `Wrap<Loud>`, isn't counted and its first clone's drop drops it; where callers are seen, a program's `pub` function's since #1112, the `Loud` is one of its instances, which #1009 counts anyway (ADR 0914, amended)",
    file: "src/lower/analysis.rs",
    find: "            if more.is_subset(&given) {\n",
    replace: "            if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^(rc_generic_given_drops|rc_dropped_where_callers_unseen|rc_dropped_in_a_closure_where_callers_unseen|rc_of_an_impls_param_dropped_last).rs$"],
  },
  {
    name: "namespace-library-not-imported",
    breaks:
      "a library's namespace a body names isn't imported, refused as a value (ADR 1037)",
    file: "src/lower/analysis.rs",
    find: "            .next()\n        {\n            let export = (from, crate::js::NAMESPACE.to_string());\n",
    replace:
      "            .find(|_| false)\n        {\n            let export = (from, crate::js::NAMESPACE.to_string());\n",
    tests: [
      "test/namespaces.test.ts",
      "-t",
      "a library's namespace is called by name",
    ],
  },
  {
    name: "superclass-unimported",
    breaks: "a class component's module never imports the class it extends, `class Counter extends Component` of no `Component`",
    file: "src/lower/analysis.rs",
    find: "            uses.imported.entry(export).or_default().insert(home);\n",
    replace: "            drop((export, home));\n",
    tests: [
      "test/class-components.test.ts",
      "-t",
      "an ES class React renders"
    ]
  },
  {
    name: "scope-of-a-counted-rc",
    breaks: "a struct an `Rc` the crate counts holds, `{ value, strong, weak }`, is a scope, its `&self` methods reading the `Rc` for the struct",
    file: "src/lower/analysis.rs",
    find: "    scopes.retain(|s| !counted.counts(tcx, tcx.type_of(s.adt).instantiate_identity().skip_normalization()));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "scope_struct"],
    snapshots: true,
  },
  {
    name: "scope-method-given-dictionary-kept",
    breaks: "a struct whose method is given a bound's dictionary is a scope, its function declared given none, which reads its function's or none (#500, #529)",
    file: "src/lower/analysis.rs",
    find: "crate::lower::traits::bounds(tcx, &foreign, given, m).is_empty()\n                && ",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "scope_struct"],
    snapshots: true,
  },
  {
    name: "scope-method-given-drop-kept",
    breaks: "a struct whose method is given a drop of its type parameter is a scope, its function declared given none, which reads its function's or none (#500, #529)",
    file: "src/lower/analysis.rs",
    find: "                && drop_params.get(&m).is_none_or(Vec::is_empty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "scope_struct"],
    snapshots: true,
  },
  {
    name: "scope-method-given-fact-kept",
    breaks: "a struct whose method is given a fact of its type parameter, its size is a scope, its function declared given none, which reads its function's or none (#500, #529)",
    file: "src/lower/analysis.rs",
    find: "                && !facts.params.contains_key(&m)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "scope_struct"],
    snapshots: true,
  },
  {
    name: "scope-method-given-location-kept",
    breaks: "a struct whose method is given its caller's location is a scope, its function declared given none, which reads its function's or none (#500, #529)",
    file: "src/lower/analysis.rs",
    find: "                && !caller_locations.contains(&m)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "scope_struct"],
    snapshots: true,
  },
  {
    name: "glob-module-unwritten",
    breaks:
      "a module whose only re-exports are globs writes no file, so its importers fail to load (#592)",
    file: "src/lower/analysis.rs",
    find: "            || super::reexports::globs_values(tcx, module);\n",
    replace: ";\n",
    tests: ["test/glob-reexports.test.ts", "-t", "a glob re-exports each name as the module it names exports it"],
  },
  {
    name: "reexported-imports-unnamed",
    breaks:
      "an import through a library's re-exporting module is named nowhere before lowering, which panics rustc reading it (rd704)",
    file: "src/lower/analysis.rs",
    find: "    for export in foreign.all_reexported() {\n",
    replace: "    for export in foreign.all_reexported().filter(|_| false) {\n",
    tests: ["test/reexports-through.test.ts", "-t", "a use through a library's module that re-exports its item imports it from there"],
  },
  {
    name: "required-use-name-unreserved",
    breaks: "a local named `require` in a module that reads a function through `require` keeps its name, so the read calls the local (rd684)",
    file: "src/lower/analysis.rs",
    find: "        uses.globals.insert(\"require\".to_string());\n",
    replace: "",
    tests: ["test/require.test.ts"],
  },
  {
    name: "library-closure-clone-undecided",
    breaks:
      "a consumer decides of a library's closure itself, not as its library's contract says: `map(|s| s.len())`'s, which rustc infers `FnMut` by `map`'s bound and its library made a plain function, is cloned apart, `$clone(f)`, which throws (#1202)",
    file: "src/lower/analysis.rs",
    find: "        if let Some(apart) = foreign.closure_cloned_apart(id) {\n            return !apart;\n        }\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "two crates: library_closures_cloned_as_made"],
  },
  {
    name: "generic-clones-unseen-by-rc-counts",
    breaks:
      "an `Rc<dyn Fn>` made of a closure cloned in generic code, `Rc::new(dup(&read))`, isn't counted, so it and `Rc::new(read)` are one JS function, `Rc::ptr_eq` `true` (#1249)",
    file: "src/lower/analysis.rs",
    find: "                (all_bodies, calls.names(), &self_handles, &cloned, library),\n",
    replace:
      "                (all_bodies, calls.names(), &self_handles, &clones::cloned_types(tcx, (&[], calls.names()), library), library),\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_fn_ptr_eq_of_a_closure_cloned_in_generic_code.rs$"],
  },
  {
    name: "std-exports-reserved-everywhere",
    breaks: "every module reserves a name for each of `@rust-js/std`'s exports, imported or not, so std's `take` of `io::take` costs a crate's own `take` its name, `take$1` (#1156)",
    file: "src/lower/analysis.rs",
    find: "    let of_a_library = |(from, _): &Export| !builtin.contains(from.as_str());\n",
    replace: "    let of_a_library = |_: &Export| !builtin.is_empty() || true;\n",
    tests: ["test/corpus.test.ts", "-t", "^(refused_drops_and_displays|consumer_drops_what_is_left).rs$"],
    snapshots: true,
  },
  {
    name: "dyn-drops-settled-in-order",
    breaks: "which `dyn Fn`s drop is settled before `dyn Any`s are known to, so a `Box<dyn Fn()>` whose closure holds a `Box<dyn Any>` of a value with a destructor never runs it (#1662)",
    file: "src/lower/analysis.rs",
    find: "        if settled {\n",
    replace: "        if settled || closure_facts.any_dyns_drop {\n",
    tests: ["test/corpus.test.ts", "-t", "^fn_dyn_holding_an_any_dyn_drops_it.rs$"],
  },
  {
    name: "held-stage-never-decided",
    breaks: "the second stage of which adapters each body makes in place is never decided, so the first reader of both, `type_fact_params`, reads a set half decided: rust-js panics, as `contains` refuses it (#1681's step iii)",
    file: "src/lower/analysis.rs",
    find: "        crate::lower::body_queries::decide_held(&recognition, bodies, (&back_stepped, keeps_capacity));\n",
    replace: "        let _ = bodies;\n",
    tests: ["test/corpus.test.ts", "-t", "^map_held_then_enumerated_from_its_back.rs$"],
  },
];
