// Mutations of src/lower/recognition/std_crates.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "std-path-through-alias",
    breaks:
      "std's items named through a crate's own name for `alloc`, `stdalloc::alloc::alloc`, aren't std's, and are refused",
    file: "src/lower/recognition/std_crates.rs",
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
    file: "src/lower/recognition/std_crates.rs",
    find: "        .find(|&std| std_crate_num(tcx, std) == Some(krate))\n",
    replace: '        .find(|&std| tcx.crate_name(krate).as_str() == ["core", "alloc", "std"][std as usize])\n',
    tests: ["test/crates.test.ts", "-t", "crate_named_core"],
  },
  {
    name: "std-path-of-any-crate",
    breaks:
      "an item of a crate named `std` that rust-js didn't compile has std's path, so its `process::exit` is std's, which ends the program, where a call of it is refused (#455, ADR 1054)",
    file: "src/lower/recognition/std_crates.rs",
    find: "    if !is_std_crate(tcx, id) {\n        return \"\";\n    }\n    let mut path = tcx.def_path_str(id);\n",
    replace: "    let mut path = tcx.def_path_str(id);\n",
    tests: ["test/crates.test.ts", "-t", "of no JS named test or std"],
  },
  {
    name: "no-std-paths-core",
    breaks:
      "a `#![no_std]` crate's std items, which rustc names `core::str::Chars`, aren't std's",
    file: "src/lower/recognition/std_crates.rs",
    find: '        Some(("core" | "alloc", rest)) => format!("std::{rest}"),',
    replace:
      '        Some(("alloc", rest)) => format!("std::{rest}"),',
    tests: ["test/crates.test.ts", "-t", "std items are std"],
  },
];
