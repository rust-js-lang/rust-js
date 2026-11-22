// Mutations of crates/rust-js-mir/src/dispatch/cells_compared.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-cell-compare-handles",
    breaks: "MIR's `p > q` of `&mut` numbers compares the handles",
    file: "crates/rust-js-mir/src/dispatch/cells_compared.rs",
    find: "            && generic_args.types().any(|t| self.is_cell(t))\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_compare"],
  },
  {
    name: "max-of-mut-refs-their-pointees",
    breaks: "`ra.max(rb)` of two `&mut i32` is `Math.max` of their handles, which prints `undefined` where native prints 2, and `*rc.min(rd) += 10` throws (#2188)",
    file: "crates/rust-js-mir/src/dispatch/cells_compared.rs",
    find: "            if let Some((call @ (OrderingCall::Max | OrderingCall::Min | OrderingCall::Clamp), _)) =\n",
    replace: "            if let Some((call @ (OrderingCall::Clamp), _)) =\n",
    tests: ["test/corpus.test.ts", "-t", "^ord_of_mut_refs.rs$"],
  },
  {
    name: "clamp-of-mut-refs-given-its-helper",
    breaks: "`clamp` of `&mut`s is given to `$clampBy` with their `PartialOrd`, which would read them as generic code's boxes, refused only as generic code where none is written (#2188)",
    file: "crates/rust-js-mir/src/dispatch/cells_compared.rs",
    find: "                if matches!(call, OrderingCall::Clamp) {\n",
    replace: "                if matches!(call, OrderingCall::Lt) {\n",
    tests: ["test/diagnostics.test.ts", "-t", "clamp of &muts to numbers is refused"],
  },
];
