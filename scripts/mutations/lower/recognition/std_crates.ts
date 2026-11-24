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
];
