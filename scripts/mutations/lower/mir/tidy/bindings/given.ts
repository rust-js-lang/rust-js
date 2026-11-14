// Mutations of src/lower/mir/tidy/bindings/given.rs (ADR 0093).
import type { Mutation } from "../../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "given-last-past-a-jump",
    breaks:
      "`node = { let node = make(); if skip { continue; } node }` gives `node` what's made where it's made, so a turn that skips still changes it",
    file: "src/lower/mir/tidy/bindings/given.rs",
    find: "        || jumps(between)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^cache_node_walk.rs$"],
  },
  {
    name: "given-last-of-a-captured",
    breaks:
      "a closure reading `node`, run before `node` is given what's made, reads what's made, not what `node` was",
    file: "src/lower/mir/tidy/bindings/given.rs",
    find: "    if early(x) || early(y) {\n",
    replace: "    if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^cache_node_walk.rs$"],
  },
  {
    name: "given-last-of-a-capture",
    breaks:
      "a closure's `x = { let x = k * 10; check(k); x }` of a variable it captures sets it before `check` unwinds, which `catch_unwind`'s caller then reads",
    file: "src/lower/mir/tidy/bindings/given.rs",
    find: "    if !kin(x, y) || !declared.contains(x.as_str()) {\n",
    replace: "    if !kin(x, y) {\n",
    tests: ["test/corpus.test.ts", "-t", "^given_last_unwinds.rs$"],
  },
  {
    name: "given-last-made-before-in-its-block-unseen",
    breaks:
      "a closure made before in the block, `const read = () => seen`, called between, reads what `seen` is given, not what it was (rd105, ADR 0853)",
    file: "src/lower/mir/tidy/bindings/given.rs",
    find: "    let captured = captured(&stmts[..given]);\n",
    replace: "    let captured = IndexSet::<String>::default();\n",
    tests: ["test/corpus.test.ts", "-t", "^cache_node_walk.rs$"],
  },
  {
    name: "given-last-made-around-unseen",
    breaks:
      "a closure made before the `if` the variable's given in, called between, reads what it's given, not what it was (rd105, ADR 0853)",
    file: "src/lower/mir/tidy/bindings/given.rs",
    find: "        made.extend(captured(std::slice::from_ref(&*stmt)));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^given_last_closure_after.rs$"],
  },
];
