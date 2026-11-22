// Mutations of crates/rust-js-lower/src/analysis/type_facts/knowing.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "knowing-steps-of-own-method",
    breaks: "a call of a compiled std struct's own method, `Fuse`'s `rfold` or `len` of it as it is, asks what its knowing steps ask, which lowering never makes of it there: a parameter, `IFused`, nothing reads (#2042)",
    file: "crates/rust-js-lower/src/analysis/type_facts/knowing.rs",
    find: "        .filter(|&ty| Some(ty) != called_on)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^map_stepped_from_its_back\\.rs$"],
    snapshots: true,
  },
  {
    name: "knowing-steps-of-any-argument",
    breaks: "a compiled std struct given where nothing steps it, `format!`'s `Debug` of a `Fuse<I>`, asks what its knowing steps ask: a parameter, `IFused`, nothing reads (#2042)",
    file: "crates/rust-js-lower/src/analysis/type_facts/knowing.rs",
    find: "        .filter(|&(_, own)| own.as_type().is_some_and(|own| bounded.contains(&own)))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^std_fuse\\.rs$"],
    snapshots: true,
  },
];
