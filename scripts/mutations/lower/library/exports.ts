// Mutations of src/lower/library/exports.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "library-changed-unwritten",
    breaks:
      "a library's contract says it changes nothing in place, so a consumer that changes what it does compiles its own of each exported function that copies one, `firstNotch` (rd606)",
    file: "src/lower/library/exports.rs",
    find: "        changed: shapes(tcx, changed.mutated.iter().chain(changed.vecs).copied()),\n",
    replace: "        changed: shapes(tcx, changed.vecs.iter().copied().filter(|_| false)),\n",
    tests: [
      "test/crates-compiled-here.test.ts",
      "-t",
      "a library's generic function is its program's",
    ],
  },
  {
    name: "library-default-type-imported-by-name",
    breaks:
      "a consumer's declarations import a library's default class by its name, `import type { Complex }`, which its module doesn't export (ADR 0210, amended)",
    file: "src/lower/library/exports.rs",
    find: '            default: crate::lower::default_exports::exported_as(tcx, id, tcx.item_name(id).as_str()) == "default",\n',
    replace: "            default: false,\n",
    tests: ["test/declarations-libraries.test.ts", "-t", "default class"],
  },
  {
    name: "held-unlisted",
    breaks:
      "a library's manifest says nothing of the bindings its modules hold, so a consumer reads each in place",
    file: "src/lower/library/exports.rs",
    find: "                export: binding.import()?.to_string(),\n",
    replace:
      "                export: binding.import().filter(|_| false)?.to_string(),\n",
    tests: ["test/cargo-link.test.ts", "-t", "holds a binding"],
  },
  {
    name: "library-dictionary-fails-unlisted",
    breaks:
      "a library's dictionary of a writer that may fail, which its consumer calls through it, `hourDisplay.fmt(hour)`, is listed as one that never does, so its error escapes the consumer's `write!` (ADR 0914, amended)",
    file: "src/lower/library/exports.rs",
    find: "                                && (tcx.associated_item_def_ids(id).iter()).any(|method| failing.contains(method))),\n",
    replace:
      "                                && (tcx.associated_item_def_ids(id).iter()).any(|_| false)),\n",
    tests: ["test/crates.test.ts", "-t", "may fail is one its consumer"],
  },
  {
    name: "library-fails-unlisted",
    breaks: "a library's writer that may fail is listed as one that never does",
    file: "src/lower/library/exports.rs",
    find: "                        failing.contains(&id)\n                            || (matches!",
    replace:
      "                        false && failing.contains(&id)\n                            || (matches!",
    tests: ["test/crates.test.ts", "-t", "may fail is one its consumer"],
  },
  {
    name: "manifest-capacity-unsaid",
    breaks:
      "a library's manifest doesn't say it keeps capacity, so its consumers don't",
    file: "src/lower/library/exports.rs",
    find: "        keeps_capacity: *keeps_capacity,\n",
    replace: "        keeps_capacity: false,\n",
    tests: ["test/crates.test.ts", "-t", "asks a Vec its capacity"],
  },
  {
    name: "library-location-unexported",
    breaks:
      "a library's function that takes its caller's location says it doesn't, and its consumers give it none",
    file: "src/lower/library/exports.rs",
    find: "                    (ItemFact::TakesLocation, caller_locations.contains(&id)),\n",
    replace:
      "                    (ItemFact::TakesLocation, false && caller_locations.contains(&id)),\n",
    tests: ["test/crates.test.ts", "-t", "two crates: track_caller"],
  },
  {
    name: "library-rest-unlisted",
    breaks:
      "a library's manifest says no function takes rest arguments, so a consumer gives its slice whole",
    file: "src/lower/library/exports.rs",
    find: "                    (ItemFact::Rest, crate::lower::bindings::takes_rest(tcx, id)),\n",
    replace: "                    (ItemFact::Rest, false),\n",
    tests: ["test/rest-parameters.test.ts", "-t", "library's rest function"],
  },
  {
    name: "library-default-listed-by-name",
    breaks:
      "a library's manifest lists a module's default by its name, which a consumer imports and the module doesn't export",
    file: "src/lower/library/exports.rs",
    find: "                .then(|| crate::lower::default_exports::exported_as(tcx, id, info.exported()))\n",
    replace:
      '                .then(|| crate::lower::default_exports::exported_as(tcx, id, info.exported()))\n                .filter(|exported| exported != "default")\n',
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "default export is imported as its default",
    ],
  },
  {
    name: "library-dropping-anys-unsaid",
    breaks:
      "a library that resumes an unwind of a guard says it makes no `dyn Any` of what drops, so its consumer's `catch_unwind` never drops the guard it caught",
    file: "src/lower/library/exports.rs",
    find: "        makes_dropping_anys: *makes_dropping_anys,\n",
    replace: "        makes_dropping_anys: false,\n",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "where the library makes one of what drops",
    ],
  },
  {
    name: "library-alias-listed-by-name",
    breaks:
      "a library's manifest lists an item its module exports as `unstable_now` alone by its name, which a consumer imports and the module doesn't export (ADR 0596, amended)",
    file: "src/lower/library/exports.rs",
    find: "                .then(|| crate::lower::default_exports::exported_as(tcx, id, info.exported()))\n",
    replace:
      '                .then(|| crate::lower::default_exports::exported_as(tcx, id, info.exported()))\n                .filter(|exported| exported == "default")\n',
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "a library's item its module exports by another name is imported by that",
    ],
  },
  {
    name: "contract-ref-cells-unwritten",
    breaks:
      "a library's contract says nothing of its `RefCell`s, so its consumer reads its plain field as a `{ value }` (ADR 0863)",
    file: "src/lower/library/exports.rs",
    find: "        ref_cells: contracts.ref_cells.clone(),\n",
    replace: "        ref_cells: Default::default(),\n",
    tests: ["test/crates.test.ts", "-t", "a library's RefCells are the values"],
  },
  {
    name: "contract-variable-unsaid",
    breaks:
      "a library's manifest doesn't say its exported cell is its module's variable, so a consumer reads its `value` (ADR 0270, amended)",
    file: "src/lower/library/exports.rs",
    find: "                    (ItemFact::Variable, variables.contains(&id)),\n",
    replace: "                    (ItemFact::Variable, false),\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "a library's exported cell",
    ],
  },
  {
    name: "library-identities-unlisted",
    breaks:
      "a library's contract lists none of its types equal only to themselves, and its consumer refuses a set of one",
    file: "src/lower/library/exports.rs",
    find: "            let mut keys: Vec<String> = (identities.iter())\n                .filter(|&&id| reachable(tcx, id))\n",
    replace:
      "            let mut keys: Vec<String> = (identities.iter())\n                .filter(|_| false)\n",
    tests: [
      "test/crates.test.ts",
      "-t",
      "equal only to itself keys its consumer",
    ],
  },
  {
    name: "library-lists-unexported",
    breaks:
      "a library's manifest lists what its JS doesn't export, an impl's method, which its consumer then imports (ADR 0914, amended)",
    file: "src/lower/library/exports.rs",
    find: "                && exported.contains(&(module_path(tcx, info.module), export_of(id, info)))\n",
    replace: "",
    tests: ["test/crates-compiled-here.test.ts", "-t", "program's, alone or a dependency"],
  },
  {
    name: "library-fn-dyns-unwritten",
    breaks:
      "a library's contract says it makes no `dyn Fn` of what drops, though it does, so its consumer drops the one it's handed as nothing (ADR 0626, amended)",
    file: "src/lower/library/exports.rs",
    find: "        makes_dropping_fn_dyns: *makes_dropping_fn_dyns,\n",
    replace: "        makes_dropping_fn_dyns: false,\n",
    tests: [
      "test/cross-crate-dyn-fns.test.ts",
      "-t",
      "a library's dyn Fn of what drops is dropped",
    ],
  },
  {
    name: "gives-argument-unlisted",
    breaks:
      "a library's contract lists none of its functions that give what they're given, so a consumer calls `eventPriorityFromI32_from(p)`",
    file: "src/lower/library/exports.rs",
    find: "            let mut keys: Vec<String> = (gives_argument.iter())\n",
    replace:
      "            let mut keys: Vec<String> = (gives_argument.iter().take(0))\n",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "transparent struct's From",
    ],
  },
  {
    name: "library-eq-by-address-unlisted",
    breaks:
      "a library's contract lists no type whose `PartialEq` alone is `ptr::eq`, so its consumer compiles the library's `updatePartialEq_eq` again and calls it, where `===` is its `==` (ADR 0864, amended)",
    file: "src/lower/library/exports.rs",
    find: "            let mut keys: Vec<String> = (eq_by_address.iter())\n",
    replace:
      "            let mut keys: Vec<String> = (eq_by_address.iter().take(0))\n",
    tests: ["test/crates.test.ts", "-t", "never \\$eq"],
  },
  {
    name: "export-named-apart-contract-apart",
    breaks:
      "another crate imports an export named apart by its module's name for it, `delete$`, which its JS doesn't export (#338)",
    file: "src/lower/library/exports.rs",
    find: "                .then(|| crate::lower::default_exports::exported_as(tcx, id, info.exported()))\n",
    replace:
      "                .then(|| crate::lower::default_exports::exported_as(tcx, id, &info.name))\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "an export its module names apart is exported by its own name",
    ],
  },
  {
    name: "library-built-unwritten",
    breaks:
      "a library's contract leaves out the structs its code makes, so its consumer takes the reconciler's fiber, which React DOM casts a node's key to, for JS's, `const old = fiber.return;` (rd221)",
    file: "src/lower/library/exports.rs",
    find: "            let mut keys: Vec<String> = built.iter().map(|&id| key(tcx, id)).collect();\n",
    replace:
      "            let mut keys: Vec<String> = built.iter().take(0).map(|&id| key(tcx, id)).collect();\n",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "a struct a library builds is the program's own data",
    ],
  },
  {
    name: "contract-arguments-unsaid",
    breaks: "a library's function that reads `js::arguments()` isn't one to its consumer, whose call leaves out its trailing `undefined`s (rd332)",
    file: "src/lower/library/exports.rs",
    find: "                    (ItemFact::Arguments, reads_arguments.functions.contains(&id)),\n",
    replace: "                    (ItemFact::Arguments, false),\n",
    tests: ["test/arguments.test.ts"],
  },
  {
    name: "contract-with-this-arguments-unsaid",
    breaks: "a library's `function::with_this` functions that read `js::arguments()` aren't in its contract, so a consumer's call of one through its `dyn Fn` leaves out its trailing `undefined`s, `Dep.counter()()` counting 0 (#553)",
    file: "src/lower/library/exports.rs",
    find: "            let mut shapes: Vec<RcShape> = (reads_arguments.with_this.iter())\n",
    replace: "            let mut shapes: Vec<RcShape> = (reads_arguments.with_this.iter().take(0))\n",
    tests: ["test/arguments.test.ts"],
  },
  {
    name: "contract-this-unsaid",
    breaks: "a library's function that reads `js::this()` isn't one to its consumer, whose call through the module's namespace gives it the namespace, and whose use of it as a value is compiled (rd683)",
    file: "src/lower/library/exports.rs",
    find: "                    (ItemFact::This, reads_arguments.this.contains(&id)),\n",
    replace: "                    (ItemFact::This, false),\n",
    tests: ["test/this.test.ts", "-t", "library"],
  },
  {
    name: "owned-by-return-unexported",
    breaks:
      "a library's contract says nothing of what its functions' returns own, so its consumer's callbacks count the library's request: refused",
    file: "src/lower/library/exports.rs",
    find: "                owned_by_return: owned_by_return.of(id),\n",
    replace: "                owned_by_return: Vec::new(),\n",
    tests: ["test/crates.test.ts", "-t", "owns its callbacks alone"],
  },
  {
    name: "nulled-params-unwritten",
    breaks:
      "a library's contract says none of its functions' own nullable parameters is `null` at each call, where its body tests one `!== null`, so a consumer gives it `undefined` (rd613)",
    file: "src/lower/library/exports.rs",
    find: "                        .filter(|&&(of, _)| of == id)\n",
    replace: "                        .filter(|_| false)\n",
    tests: ["test/cross-crate-items.test.ts", "-t", "nullable parameter its calls give null"],
  },
  {
    name: "associated-facts-unexported",
    breaks: "a library's manifest says none of what its traits' dictionaries carry of their associated types, so a consumer compiling its `size_of::<A::Item>()` is refused (#476)",
    file: "src/lower/library/exports.rs",
    find: "                .filter(|&(&id, _)| id.is_local() && tcx.def_kind(id) == rustc_hir::def::DefKind::AssocTy)\n",
    replace: "                .filter(|_| false)\n",
    tests: ["test/crates.test.ts", "-t", "item_size_of_instance"],
  },
  {
    name: "keyed-contract-unwritten",
    breaks: "a library's contract lists none of its keyed enums, so its consumer tags them (ADR 1071)",
    file: "src/lower/library/exports.rs",
    find: "                .filter(|&&id| id.is_local() && reachable(tcx, id))\n                .map(|&id| key(tcx, id))\n                .collect();\n            keys.sort();\n            keys\n        },\n        identities: {\n",
    replace: "                .filter(|_| false)\n                .map(|&id| key(tcx, id))\n                .collect();\n            keys.sort();\n            keys\n        },\n        identities: {\n",
    tests: ["test/flattened-keyed.test.ts", "-t", "contract"],
  },
  {
    name: "reexported-unsaid",
    breaks:
      "a library's contract says none of its modules' re-exports, so a consumer naming an item through ReactFiberReconciler imports it from its home, a second import (rd704)",
    file: "src/lower/library/exports.rs",
    find: "    let mut reexported: Vec<Reexported> = (crate::lower::reexports::reexported_items(tcx, functions).into_iter())\n",
    replace: "    let mut reexported: Vec<Reexported> = (crate::lower::reexports::reexported_items(tcx, functions).into_iter().filter(|_| false))\n",
    tests: ["test/reexports-through.test.ts", "-t", "a use through a library's module that re-exports its item imports it from there"],
  },
  {
    name: "reexported-of-the-root",
    breaks:
      "a library's contract says its root's re-exports, of a root that writes no JS where its package's entry re-exports them, so its consumer fails, `dependency export references a missing module` (rd704, ADR 1008)",
    file: "src/lower/library/exports.rs",
    find: "            !module.is_top_level_module() && exported.contains(&(module_path(tcx, *module), export.clone()))\n",
    replace: "            exported.contains(&(module_path(tcx, *module), export.clone()))\n",
    tests: ["test/libraries.test.ts", "-t", "entry re-exports"],
  },
  {
    name: "forwarded-method-uncontracted",
    breaks:
      "a library's namespace method that's the function it passes its parameters on to is in no contract, so another crate's call of it through the namespace's value is refused (#596, rd728)",
    file: "src/lower/library/exports.rs",
    find: "        Some(crate::lower::classes::Member::Namespace) => crate::lower::classes::property(tcx, id),\n",
    replace: "        Some(crate::lower::classes::Member::Namespace) if false => crate::lower::classes::property(tcx, id),\n",
    tests: ["test/method-forwarding.test.ts", "-t", "another crate calls a library's forwarding method"],
  },
  {
    name: "partial-cmp-is-cmp-unlisted",
    breaks: "a library's contract lists none of its impls whose `partial_cmp` is `Some(self.cmp(other))`, so a consumer's `partial_cmp` of a derived one compares the fields' `partial_cmp`s (#964)",
    file: "src/lower/library/exports.rs",
    find: "                .filter(|&imp| crate::lower::ordering::partial_cmp_is_cmp(tcx, imp))\n",
    replace: "                .filter(|_| false)\n",
    tests: ["test/crates.test.ts", "-t", "derived_partial_cmp_is_cmp"],
  },
  {
    name: "field-giving-method-listed",
    breaks:
      "a library's class's trait method that gives its field, which has no JS of its own (rd304), is listed in its manifest, `Holder.slot`, a member its consumer calls and finds undefined (#1032)",
    file: "src/lower/library/exports.rs",
    find: "                && !crate::lower::classes::gives_its_field(tcx, id)\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "lending a library's quiet field"],
  },
  {
    name: "super-item-unexported",
    breaks: "a library's manifest doesn't say what its traits' dictionaries carry of `Iterator::Item`, so a consumer's impls carry none and its compile of the default is refused (#990)",
    file: "src/lower/library/exports.rs",
    find: "            for (&carrier, facts) in contracts.carried_super.iter().filter(|&(&id, _)| id.is_local()) {\n",
    replace: "            for (&carrier, facts) in contracts.carried_super.iter().filter(|_| false) {\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_default_item_layouts"],
  },
  {
    name: "library-closure-clone-said-inverted",
    breaks: "a library's contract says each closure it clones apart is its own clone, and the other way round (#1202)",
    file: "src/lower/library/exports.rs",
    find: "                    key: key(tcx, id),\n                    apart,\n",
    replace: "                    key: key(tcx, id),\n                    apart: !apart,\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_closures_cloned_as_made"],
  },
  {
    name: "manifest-dyn-debug-hex-dropped",
    breaks: "a library's manifest says it shows no `dyn Debug` in hex, so its consumer's `{:x?}` of one is decimal (#1335)",
    file: "src/lower/library/exports.rs",
    find: "        dyn_debug_hex: dyn_debug_shown.hex,\n",
    replace: "        dyn_debug_hex: false,\n",
    tests: ["test/crates.test.ts", "-t", "library_shows_a_consumers_dyn_debug_with_options"],
  },
  {
    name: "library-modules-unlisted",
    breaks:
      "a library's contract lists none of its modules, so a consumer's `pub use` of one, rand_chacha's `pub use rand_core;`, re-exports nothing (#1420)",
    file: "src/lower/library/exports.rs",
    find: "        .filter(|&id| tcx.def_kind(id) == rustc_hir::def::DefKind::Mod)\n",
    replace: "        .filter(|&id| false && tcx.def_kind(id) == rustc_hir::def::DefKind::Mod)\n",
    tests: ["test/glob-reexports.test.ts", "-t", "library's module is its namespace"],
  },
  {
    name: "drops-any-unexported",
    breaks:
      "a library's contract says none of its values drops, so its consumer, none of whose own does, gives the library's generic impl's method it compiles no drops: `drop Up 3` never printed (#1591)",
    file: "src/lower/library/exports.rs",
    find: "        drops_any: *drops_any,\n",
    replace: "        drops_any: { let _ = drops_any; false },\n",
    tests: ["test/crates.test.ts", "-t", "library_extend_drops_the_crates_iterator|library_extend_given_a_middle_crates_iterator"],
  },
];
