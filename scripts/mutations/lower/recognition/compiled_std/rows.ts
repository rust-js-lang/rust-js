// Mutations of crates/rust-js-lower/src/recognition/compiled_std/rows.rs (ADR 0093): std's
// catalog of compiled bodies, each row resolved once (#1681).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "net-addresses-uncompiled",
    breaks: "`core::net`'s addresses aren't compiled into `@rust-js/std`, so `Ipv4Addr::new` is refused again (#1420)",
    file: "crates/rust-js-lower/src/recognition/compiled_std/rows.rs",
    find: "        .chain(COMPILED_STD_NET.iter().map(|&row| (row, methods(row))))\n",
    replace: "",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std's bodies"],
  },
  {
    name: "catalog-private-trait-unfound",
    breaks: "a row of a trait std keeps private beside its type, `Fuse`'s `FuseImpl`, resolves to nothing, an internal error every compile of std's bodies",
    file: "crates/rust-js-lower/src/recognition/compiled_std/rows.rs",
    find: "                    .or_else(|| private_trait(trait_name))\n",
    replace: "",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std's bodies"],
  },
  {
    name: "io-rows-without-alloc",
    breaks: "a `#![no_std]` crate without `alloc` resolves `io`'s rows, whose types `core::io` names but whose `Read` impls are `alloc`'s: an internal error, every such crate refused",
    file: "crates/rust-js-lower/src/recognition/compiled_std/rows.rs",
    find: "    let io = (COMPILED_STD_IO.iter().filter(|_| alloc).map(|&row| (row, methods(row))))\n",
    replace: "    let io = (COMPILED_STD_IO.iter().filter(|_| alloc || true).map(|&row| (row, methods(row))))\n",
    tests: ["test/std-paths.test.ts"],
  },
  {
    name: "catalog-impls-of-another-type",
    breaks: "a row's impls are asked of another type's bucket, `str`'s, so `Map`'s `Iterator` impl and every other row of a trait resolve to nothing, an internal error every compile of std's bodies (#2312)",
    file: "crates/rust-js-lower/src/recognition/compiled_std/rows.rs",
    find: "    let bucket = impls.non_blanket_impls().get(&SimplifiedType::Adt(adt));\n",
    replace: "    let bucket = impls.non_blanket_impls().get(&SimplifiedType::Str);\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std's bodies"],
  },
];
