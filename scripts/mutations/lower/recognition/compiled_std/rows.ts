// Mutations of src/lower/recognition/compiled_std/rows.rs (ADR 0093): std's
// catalog of compiled bodies, each row resolved once (#1681).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "net-addresses-uncompiled",
    breaks: "`core::net`'s addresses aren't compiled into `@rust-js/std`, so `Ipv4Addr::new` is refused again (#1420)",
    file: "src/lower/recognition/compiled_std/rows.rs",
    find: "    .chain(\n        COMPILED_STD_NET\n            .iter()\n            .map(|&row| (row, methods(StdCrate::Core, \"net\", row))),\n    )\n",
    replace: "",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std's bodies"],
  },
  {
    name: "catalog-private-trait-unfound",
    breaks: "a row of a trait std keeps private beside its type, `Fuse`'s `FuseImpl`, resolves to nothing, an internal error every compile of std's bodies",
    file: "src/lower/recognition/compiled_std/rows.rs",
    find: "                    .or_else(|| private_trait(trait_name))\n",
    replace: "",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std's bodies"],
  },
];
