// Mutations of crates/rust-js-lower/src/analysis/intervals/keys.rs, a map's key held
// where its `contains_key` showed it (ADR 1137, #1672 M3, step 5).
import type { Mutation } from "../../../../mutations";

const file = "crates/rust-js-lower/src/analysis/intervals/keys.rs";
const tests = ["test/intervals-shadow.test.ts", "-t", "contains_key"];

export const mutations: Mutation[] = [
  {
    name: "keys-tested-unseen",
    breaks: "a `contains_key(&k)` switched on shows nothing, so `errors[id]` under `errors.contains_key(id)` keeps its check, a call-bounds line",
    file,
    find: "                after.has_keys.insert(local, held);\n",
    replace: "",
    tests,
  },
  {
    name: "key-index-not-given-back",
    breaks: "an `m[&k]` that gave its value shows nothing, so a second `m[&k]` keeps its check, no proved-call-bounds line",
    file,
    find: "                after.keys.insert(held);\n",
    replace: "",
    tests,
  },
  {
    name: "key-held-of-any-key",
    breaks: "a key whose lookup a call may change, of a `Cell` its `Hash` reads, is held past `contains_key`, so `m[k]` past `bump()` is taken in bounds, a proved-call-bounds line, and would read `undefined` where Rust panics",
    file,
    find: "        (held_keys && key_ty.is_some_and(stays)).then_some(())?;\n",
    replace: "",
    tests,
  },
];
