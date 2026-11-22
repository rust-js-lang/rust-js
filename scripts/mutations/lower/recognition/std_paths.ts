// Mutations of crates/rust-js-lower/src/recognition/std_paths.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

const file = "crates/rust-js-lower/src/recognition/std_paths.rs";

export const mutations: Mutation[] = [
  {
    name: "std-paths-std-alone",
    breaks: "a row is found through `std`'s modules alone, so a `#![no_std]` crate's `panic::Location`, core's, isn't std's",
    file,
    find: "        let found = [StdCrate::Std, StdCrate::Core, StdCrate::Alloc]\n",
    replace: "        let found = [StdCrate::Std]\n",
    tests: ["test/std-paths.test.ts"],
  },
  {
    name: "std-paths-none",
    breaks: "no item is std's of the table, so `env::VarError` is a crate's enum, not std's `\"NotPresent\"`",
    file,
    find: "    std_paths(tcx).by_id.get(&id).copied()\n",
    replace: "    std_paths(tcx).by_id.get(&id).copied().filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^env_vars.rs$"],
  },
  {
    name: "std-paths-primitive-method-last",
    breaks: "a row of a primitive's method is resolved as its path without `<impl str>` first, the free function `std::str::from_utf8`, so two rows are one item, an internal error every compile",
    file: "crates/rust-js-lower/src/recognition/std_paths.rs",
    find: "        let found = primitive_method(tcx, item.path()).or(found);\n",
    replace: "        let found = found.or_else(|| primitive_method(tcx, item.path()));\n",
    tests: ["test/std-paths.test.ts"],
  },
  {
    name: "method-of-a-slice-a-generic",
    breaks: "an impl of a type of a slice, `NonNull<[T]>`, or a raw pointer to one, `*const [T]`, is taken for its generic one's, so `NonNull::<[T]>::len` and `<*const [T]>::len` are asked of `NonNull<T>`'s and `*const T`'s methods and no slice's length",
    file: "crates/rust-js-lower/src/recognition/std_paths.rs",
    find: "        ty::Slice(_) => Some(true),\n        _ if crate::generic_facts::is_impl_parameter(ty) => Some(false),\n",
    replace: "        ty::Slice(_) => Some(false),\n        _ if crate::generic_facts::is_impl_parameter(ty) => Some(false),\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_pointers.rs$"],
  },
  {
    name: "method-of-a-non-null-a-raw-pointer",
    breaks: "a `NonNull`'s impl is taken for a `*mut T`'s, as `raw_pointee` takes it for one, so its `as_ref` is a raw pointer's, which may be `None`, not a `NonNull`'s, never",
    file: "crates/rust-js-lower/src/recognition/std_paths.rs",
    find: "    if !matches!(ty.kind(), ty::Adt(..))\n        && let Some((pointee, mutability))",
    replace: "    if true\n        && let Some((pointee, mutability))",
    tests: ["test/corpus.test.ts", "-t", "^non_null.rs$"],
  },
  {
    name: "std-modules-of-std-alone",
    breaks: "a module a recognizer asks what's under is `std`'s alone, not `core`'s or `alloc`'s of its path, so of a crate that loads no `std` nothing is under `std::hint`, and `core::hint::black_box(x)` is no longer `x`",
    file: "crates/rust-js-lower/src/recognition/std_paths.rs",
    find: "                    let ids = [StdCrate::Std, StdCrate::Core, StdCrate::Alloc]\n",
    replace: "                    let ids = [StdCrate::Std]\n",
    tests: ["test/std-paths.test.ts"],
  },
  {
    name: "std-path-under-its-parent-alone",
    breaks: "an item is under a module only where it's its parent, so `fmt::rt::Argument`'s are under `std::fmt::rt` and not `std::fmt`: no formatting of std's",
    file: "crates/rust-js-lower/src/recognition/std_paths.rs",
    find: "        && std::iter::successors(std_path_parent(tcx, id), |&at| std_path_parent(tcx, at))\n",
    replace: "        && std::iter::successors(std_path_parent(tcx, id), |_| None)\n",
    tests: ["test/corpus.test.ts", "-t", "^(fs_dirs_and_metadata|char_from_code|fs_write_only_fails).rs$"],
  },
  {
    name: "std-item-in-its-own-module",
    breaks: "an item is in a module of std's where it's defined in it, not where std's path writes it: `core::iter::adapters::map::Map` is in no `std::iter`, so no adapter passes a step on, `fuse().count()` of a crate's iterator no longer its own",
    file: "crates/rust-js-lower/src/recognition/std_paths.rs",
    find: "    (super::is_std_crate(tcx, id) && std_path_parent(tcx, id).is_some_and(|at| is_std_module(tcx, at, module)))\n",
    replace: "    (super::is_std_crate(tcx, id) && tcx.opt_parent(id).is_some_and(|at| is_std_module(tcx, at, module)))\n",
    tests: ["test/corpus.test.ts", "-t", "^(adapters_over_own_next_that_panics|adapters_own_nth_fold|adapters_own_work_that_panics).rs$"],
    snapshots: true,
  },
  {
    name: "std-modules-without-slice",
    breaks: "`std::slice`, a row asked of as a module, is no row of `STD_MODULES`, so nothing would be under it, a slice's searches no slice's: an internal error at the first question, not a wrong answer",
    file: "crates/rust-js-lower/src/recognition/std_paths.rs",
    find: "    StdPath::SliceMod,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^slice_methods.rs$"],
  },
  {
    name: "std-path-parent-defined-in",
    breaks: "std's path of an item is the module it's defined in, not the one std writes it in, so `core::iter::Take`, defined in `core::iter::adapters::take`, isn't std's `iter::Take` and drops nothing it steps (#2314)",
    file: "crates/rust-js-lower/src/recognition/std_paths.rs",
    find: "    std_visible_parents(tcx)\n        .get(&id)\n        .copied()\n        .or_else(|| tcx.opt_parent(id))\n",
    replace: "    tcx.opt_parent(id)\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_order_through_take_adapters.rs$"],
    snapshots: true,
  },
];
