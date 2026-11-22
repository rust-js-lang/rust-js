// Mutations of crates/rust-js-program/src/library.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "library-keyed-by-name",
    breaks:
      "a library is known by its crate's name alone, so two releases of one crate in one graph, cfg-if 0.1 and 1.0, are one library, refused as a duplicate (#1130)",
    file: "crates/rust-js-program/src/library.rs",
    find: '    format!("{name}-{crate_id}")\n',
    replace: "    let _ = crate_id;\n    name.to_string()\n",
    tests: ["test/sysroot-names.test.ts", "-t", "two releases of one crate"],
  },
  {
    name: "contract-counts-unread",
    breaks: "a library's contract read as counting every `Rc` whatever it says, so its consumer counts what it hands as a box (ADRs 0844, 0852)",
    file: "crates/rust-js-program/src/library.rs",
    find: "            counted_rcs,\n            with_this_arguments,\n            changed,\n            ref_cells,\n            identities: ",
    replace: "            counted_rcs: { let _ = counted_rcs; vec![RcShape { of: None, args: Vec::new() }] },\n            with_this_arguments,\n            changed,\n            ref_cells,\n            identities: ",
    tests: ["test/crates.test.ts", "-t", "two crates: rc_counted_by_its_library_alone$"],
  },
  {
    name: "manifest-writers-options-dropped",
    breaks: "a library's manifest doesn't say its writers take options: a consumer gives them none",
    file: "crates/rust-js-program/src/library.rs",
    find: "            writes_may_fail,\n            writers_take_options,\n            debug_hex,\n            dyn_debug_pretty,\n            dyn_debug_hex,\n            reads_formatter_options: ",
    replace: "            writes_may_fail,\n            writers_take_options: { let _ = writers_take_options; false },\n            debug_hex,\n            dyn_debug_pretty,\n            dyn_debug_hex,\n            reads_formatter_options: ",
    tests: ["test/crates.test.ts", "-t", "as its own crate takes them"],
  },
  {
    name: "library-default-imported-by-name",
    breaks: "a library's module's default is imported by its name, `import { noop }`, which its module doesn't export",
    file: "crates/rust-js-program/src/library.rs",
    find: "        self.exported_as.clone().unwrap_or_else(|| self.export.clone())\n",
    replace: "        self.export.clone()\n",
    tests: ["test/cross-crate-items.test.ts", "-t", "default export is imported as its default"],
  },
  {
    name: "library-dropping-anys-unread",
    breaks: "a consumer reads no library's `makes_dropping_anys`, so its `catch_unwind` never drops the guard a library resumed",
    file: "crates/rust-js-program/src/library.rs",
    find: "            makes_dropping_anys,\n            makes_dropping_errors,\n            makes_dropping_fn_dyns,\n            dictionary_consts: ",
    replace: "            makes_dropping_anys: { let _ = makes_dropping_anys; false },\n            makes_dropping_errors,\n            makes_dropping_fn_dyns,\n            dictionary_consts: ",
    tests: ["test/cross-crate-items.test.ts", "-t", "where the library makes one of what drops"],
  },
  {
    name: "contract-ref-cells-unread",
    breaks: "a library's contract's `RefCell`s are read as none, so its consumer reads its plain field as a `{ value }` (ADR 0863)",
    file: "crates/rust-js-program/src/library.rs",
    find: "            ref_cells,\n            identities: ",
    replace: "            ref_cells: { let _ = ref_cells; Default::default() },\n            identities: ",
    tests: ["test/crates.test.ts", "-t", "a library's RefCells are the values"],
  },
  {
    name: "library-fn-dyns-unread",
    breaks: "a consumer reads no library's `makes_dropping_fn_dyns`, so a `dyn Fn` of what drops a library hands it is dropped as nothing (ADR 0626, amended)",
    file: "crates/rust-js-program/src/library.rs",
    find: "            makes_dropping_anys,\n            makes_dropping_errors,\n            makes_dropping_fn_dyns,\n            dictionary_consts: ",
    replace: "            makes_dropping_anys,\n            makes_dropping_errors,\n            makes_dropping_fn_dyns: { let _ = makes_dropping_fn_dyns; false },\n            dictionary_consts: ",
    tests: ["test/cross-crate-dyn-fns.test.ts", "-t", "a library's dyn Fn of what drops is dropped"],
  },
  {
    name: "associated-facts-unread",
    breaks: "a dependency's manifest's facts of its traits' associated types are dropped as it's read, so a consumer compiling its `size_of::<A::Item>()` is refused (#476)",
    file: "crates/rust-js-program/src/library.rs",
    find: "            associated: (associated.iter())\n                .filter(|associated| associated.carrier.is_none())\n",
    replace: "            associated: (associated.iter())\n                .filter(|_| false)\n",
    tests: ["test/crates.test.ts", "-t", "item_size_of_instance"],
  },
  {
    name: "super-item-unread-from-manifest",
    breaks: "a consumer reads none of a library's manifest's `carrier` entries, so its impls carry none and its compile of the default is refused (#990)",
    file: "crates/rust-js-program/src/library.rs",
    find: "            carried_super: carried_super(&associated),\n",
    replace: "            carried_super: IndexMap::default(),\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_default_item_layouts"],
  },
  {
    name: "library-cloned-closures-unread",
    breaks:
      "a consumer reads none of a library's manifest's `cloned_closures`, so it decides of a library's closure itself: one rustc infers `FnMut` by `map`'s bound, `map(|s| s.len())`, which its library made a plain function, is cloned apart, `$clone(f)`, which throws (#1202)",
    file: "crates/rust-js-program/src/library.rs",
    find: "            cloned_closures: (cloned_closures.into_iter()).map(|c| (c.key, c.apart)).collect(),\n",
    replace: "            cloned_closures: { let _ = cloned_closures; IndexMap::default() },\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_closures_cloned_as_made"],
  },
  {
    name: "std-library-not-builtin",
    breaks:
      "`@rust-js/std` is read as a library like any other, not the compiler's own, so a library's manifest lists it among those its consumers must be given (ADR 0561)",
    file: "crates/rust-js-program/src/library.rs",
    find: "            builtin: builtin.is_some(),\n",
    replace: "            builtin: false,\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_closures_cloned_as_made"],
  },
  {
    name: "library-dropping-errors-unread",
    breaks: "a consumer reads no library's `makes_dropping_errors`, so a `Box<dyn Error>` of what drops a library hands it is dropped as nothing (ADR 0462, amended)",
    file: "crates/rust-js-program/src/library.rs",
    find: "            makes_dropping_anys,\n            makes_dropping_errors,\n            makes_dropping_fn_dyns,\n            dictionary_consts: ",
    replace: "            makes_dropping_anys,\n            makes_dropping_errors: { let _ = makes_dropping_errors; false },\n            makes_dropping_fn_dyns,\n            dictionary_consts: ",
    tests: ["test/crates.test.ts", "-t", "two crates: library_makes_a_dropping_dyn_error"],
  },
  {
    name: "drops-any-missing-drop-free",
    breaks:
      "a contract a compiler wrote before `drops_any` reads as a library none of whose values drops, so its consumer gives generic code no drops (#1591)",
    file: "crates/rust-js-program/src/library.rs",
    find: "    #[serde(default = \"made_before\")]\n    pub drops_any: bool,\n",
    replace: "    #[serde(default)]\n    pub drops_any: bool,\n",
    unitTests: "library::tests::a_contract_without_drops_any_may_drop",
  },
];
