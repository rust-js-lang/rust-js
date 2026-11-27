// Mutations of crates/rust-js-lower/src/recognition/std_crates.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "std-path-through-alias",
    breaks:
      "std's items named through a crate's own name for `alloc`, `stdalloc::alloc::alloc`, aren't std's, and are refused",
    file: "crates/rust-js-lower/src/recognition/std_crates.rs",
    find: "        path = ty::print::with_no_visible_paths!(tcx.def_path_str(id));\n",
    replace: "",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "crate's own name for alloc",
    ],
  },
  {
    name: "std-crate-by-name",
    breaks:
      "std's crates are known by their names again: a program's crate named `core` is std's, so `core::double(21)` is refused, and its `hint::black_box` is std's, its argument (#455, ADR 1054)",
    file: "crates/rust-js-lower/src/recognition/std_crates.rs",
    find: "        .find(|&std| std_crate_num(tcx, std) == Some(krate))\n",
    replace: '        .find(|&std| tcx.crate_name(krate).as_str() == ["core", "alloc", "std"][std as usize])\n',
    tests: ["test/crates-pairs-3.test.ts", "-t", "crate_named_core"],
  },
  {
    name: "std-path-of-any-crate",
    breaks:
      "an item of no std crate is given a path, so a crate's own impl of a std trait that a recogniser takes, `Whole`'s `read_to_end`, is listed as recognised and left out by `--without-recognizer` as std's (#455, ADR 1054)",
    file: "crates/rust-js-lower/src/recognition/std_crates.rs",
    find: "    if !is_std_crate(tcx, id) {\n        return \"\";\n    }\n    let mut path = tcx.def_path_str(id);\n",
    replace: "    let mut path = tcx.def_path_str(id);\n",
    tests: ["test/tidy-stages.test.ts", "-t", "what a compile recognised is std's functions"],
  },
  {
    name: "no-std-paths-core",
    breaks:
      "a `#![no_std]` crate's std items, which rustc names `core::str::Chars`, aren't std's",
    file: "crates/rust-js-lower/src/recognition/std_crates.rs",
    find: '        Some(("core" | "alloc", rest)) => format!("std::{rest}"),',
    replace:
      '        Some(("alloc", rest)) => format!("std::{rest}"),',
    tests: ["test/crates.test.ts", "-t", "std items are std"],
  },
  {
    name: "library-identity-of-its-own",
    breaks: "a library's type is branded by an identity the consumer makes of the library's name, not its manifest's, so a consumer's copy of its generic code brands a value the library refuses (ADR 1196, amended)",
    file: "crates/rust-js-lower/src/recognition/std_crates.rs",
    find: "        Some(identity) => identity.clone(),\n",
    replace: "        Some(_) => rust_js_program::library::crate_identity(tcx.crate_name(krate).as_str(), false, &[]),\n",
    tests: ["test/boundary-brands.test.ts", "-t", "a library's sealed type its consumer clones"],
  },
  {
    name: "shared-identities-unrefused",
    breaks: "a program of two crates of one identity compiles, their types' brands one, so one's value passes for the other's (ADR 1196, amended)",
    file: "crates/rust-js-lower/src/recognition/std_crates.rs",
    find: "        match seen.get(&identity) {\n",
    replace: "        match seen.get(&identity).filter(|_| false) {\n",
    tests: ["test/boundary-brands.test.ts", "-t", "a program of two crates of one identity is refused"],
  },
  {
    name: "rustc-alone-crate-unidentified",
    breaks: "a crate rustc alone built, indexmap's whose generic code a consumer compiles, has no identity, so a brand of its `IndexMap` the consumer makes panics the compile (#2403)",
    file: "crates/rust-js-lower/src/recognition/std_crates.rs",
    find: "        None => format!(\"{}#{:016x}\", tcx.crate_name(krate), tcx.stable_crate_id(krate).as_u64()),\n",
    replace: "        None => panic!(\"{} has no identity\", tcx.crate_name(krate)),\n",
    tests: ["test/bound-maps.test.ts", "-t", "indexmap's maps run as std's"],
  },
];
