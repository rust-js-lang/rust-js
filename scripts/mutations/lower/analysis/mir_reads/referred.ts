// crates/rust-js-lower/src/analysis/mir_reads/referred.rs's mutations (#2299 moved them with their code).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-promoted-const-unseen",
    breaks:
      "a promoted `&HEADERS` is the constant's whole value, written where it's read, not `HEADERS[0]`",
    file: "crates/rust-js-lower/src/analysis/mir_reads/referred.rs",
    find: "    (Some(local) == referred).then_some(c)\n",
    replace: "    (Some(local) == referred).then_some(c).filter(|_| false)\n",
    tests: ["test/compiler.test.ts", "-t", "the report's JS: function values"],
  },
];
