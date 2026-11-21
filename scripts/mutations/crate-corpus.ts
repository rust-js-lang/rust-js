// Mutations of scripts/crate-corpus.ts: what the crate corpus measures and
// holds each crate to.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "crate-labels-unlisted-pass",
    breaks: "a label MIR made in the crate corpus's JS that isn't listed under #2215 passes the crate corpus's check, unseen, as std's did before the label rule read it (#2215)",
    file: "scripts/crate-corpus.ts",
    find: "    if (!listed.has(label)) problems.push(",
    replace: "    if (false) problems.push(",
    tests: ["test/ci.test.ts", "-t", "the crate corpus's labels MIR made are listed"],
  },
];
