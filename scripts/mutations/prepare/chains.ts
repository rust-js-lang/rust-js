// Mutations of src/prepare/chains.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "chained-never",
    breaks: "two places set the same variable stay two statements, `hook.baseState = initialState; hook.memoizedState = initialState;`, not React's `hook.memoizedState = hook.baseState = initialState;`",
    file: "src/prepare/chains.rs",
    find: "        if run == 2 {\n",
    replace: "        if run == 2 && false {\n",
    tests: ["test/corpus.test.ts", "-t", "^assignments_chained.rs$"],
    snapshots: true,
  },
  {
    name: "chained-of-any-value",
    breaks: "two places set the same value made of more than a name, `n + 1`, are one statement, which evaluates it once where Rust's does twice",
    file: "src/prepare/chains.rs",
    find: "        _ => false,\n    }\n}\n\n/// Whether `target`",
    replace: "        _ => value.kind == again.kind || matches!((&value.kind, &again.kind), (ExprKind::Binary(..), ExprKind::Binary(..))),\n    }\n}\n\n/// Whether `target`",
    tests: ["test/corpus.test.ts", "-t", "^assignments_chained.rs$"],
    snapshots: true,
  },
  {
    name: "chained-other-variable",
    breaks: "`hook.baseState = a; hook.memoizedState = b;` is `hook.memoizedState = hook.baseState = a`, `b` lost",
    file: "src/prepare/chains.rs",
    find: "        (ExprKind::Var(x), ExprKind::Var(y)) => x == y,\n",
    replace: "        (ExprKind::Var(_), ExprKind::Var(_)) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^assignments_chained.rs$"],
  },
  {
    name: "chained-variable-read-unseen",
    breaks: "`current = node; current.next = node;` is `current.next = current = node`, which sets the old `current`'s `next`",
    file: "src/prepare/chains.rs",
    find: "        ExprKind::Var(name) => object.mentions_var(name),\n",
    replace: "        ExprKind::Var(name) => false && object.mentions_var(name),\n",
    tests: ["test/corpus.test.ts", "-t", "^assignments_chained.rs$"],
  },
  {
    name: "chained-field-read-unseen",
    breaks: "`pinned.current = node; pinned.current.next = node;` is `pinned.current.next = pinned.current = node`, which sets the old `current`'s `next`",
    file: "src/prepare/chains.rs",
    find: "            object.each(&mut |e| reads |= matches!(&e.kind, ExprKind::Member(_, field) if field == set));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^assignments_chained.rs$"],
  },
  {
    name: "chained-setter-unseen",
    breaks: "a binding's two setters given the same value are one statement, `globalThis.left = globalThis.top = to`, though a setter may do anything",
    file: "src/prepare/chains.rs",
    find: "        ExprKind::Member(object, _) => !target.facts.has(Known::Setter) && path(object),\n",
    replace: "        ExprKind::Member(object, _) => path(object),\n",
    tests: ["test/lowering-chained.test.ts"],
  },
  {
    name: "chained-runs-of-any-length",
    breaks: "a run of three places set alike, drop flags' `a$live = false;` each, has its first two chained, `b = a = false; c = false;`",
    file: "src/prepare/chains.rs",
    find: "        if run == 2 {\n",
    replace: "        if run >= 2 {\n",
    tests: ["test/corpus.test.ts", "-t", "^assignments_chained.rs$"],
    snapshots: true,
  },
];
