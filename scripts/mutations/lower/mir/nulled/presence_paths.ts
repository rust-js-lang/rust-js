// Mutations of crates/rust-js-mir/src/nulled/presence_paths.rs (ADR 0093): an
// `Option` present where control flow joins, each path to the read having
// filled it or taken a test's present edge (ADR 0620, amended).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "proved-present-join-unread",
    breaks: "an option filled on all paths to a join still unwraps at runtime",
    file: "crates/rust-js-mir/src/nulled/presence_paths.rs",
    find: "predecessors[read].len() < 2",
    replace: "predecessors[read].len() < usize::MAX",
    tests: ["test/lowering-nullable-truthy.test.ts", "-t", "filled on its missing branch"],
  },
];
