// Mutations of src/lower/mir/tidy/logicals/chains.rs (ADR 0093).
import type { Mutation } from "../../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "chain-in-blocks-unfolded",
    breaks:
      "React's attachSuspenseRetryListeners' let chain with an `else` inside an `if`, `if (lanes != null && root != null) { .. } else { throw .. }`, tests that break `bb23`",
    file: "src/lower/mir/tidy/logicals/chains.rs",
    find: "        if !chain_within(body, label) {\n",
    replace: "        if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^let_chain_else_inside_if.rs$"],
    snapshots: true,
  },
  {
    name: "block-chain-unfolded",
    breaks:
      "a let chain with an `else` whose binding is between its tests, `if (DEV) { const readable = ..; if (readable != null) { .. } }`, is left nested, the tail after it copied into an early `return`, not `if (DEV && options?.debugChannel?.readable != null) { .. } else { .. }` (rd614 b)",
    file: "src/lower/mir/tidy/logicals/chains.rs",
    find: "        if let Some((0, chain)) = chained(body, label)\n",
    replace: "        if let Some((0, chain)) = None::<(usize, Vec<Stmt>)>\n",
    tests: ["test/corpus.test.ts", "-t", "^let_chain_else_of_an_optional_read.rs$"],
    snapshots: true,
  },
  {
    name: "block-chain-of-no-binding",
    breaks:
      "a block's chain of no binding, a flag's, `const value = m.TAG === \"Rows\" && ..`, is made an `if` of its `else`, `let tmp; if (..) { tmp = false; } else { tmp = true; }`, before `flag values` sees it (rd614 b)",
    file: "src/lower/mir/tidy/logicals/chains.rs",
    find: "            && chain != *body\n",
    replace: "            && true\n",
    tests: ["test/corpus.test.ts", "-t", "^flag_of_a_guarded_match.rs$"],
    snapshots: true,
  },
  {
    name: "sunk-binding-untested",
    breaks:
      "a let chain's binding moved into its body is read by the test before it's declared, `if (a && r != null) { const r = ..`",
    file: "src/lower/mir/tidy/logicals/chains.rs",
    find: "    if let Some(tested) = test.substitute(&read) {\n",
    replace: "    if let Some(tested) = None::<Expr> {\n",
    tests: ["test/corpus.test.ts", "-t", "^let_chain_else_inside_if.rs$"],
  },
  {
    name: "chain-bindings-sunk-past-calls",
    breaks:
      "a binding of a let chain whose later test calls what may change it, `if let Some(n) = COUNT.get() && bump()`, is read after the call",
    file: "src/lower/mir/tidy/logicals/chains.rs",
    find: "        && !calls(first)\n",
    replace: "        && true\n",
    tests: ["test/corpus.test.ts", "-t", "^let_chain_else_inside_if.rs$"],
  },
  {
    name: "chain-body-calls-counted",
    breaks:
      "a let chain whose body calls, React's attachSuspenseRetryListeners' restorePendingUpdaters, is taken for one whose tests call, its bindings kept and `bb27` left (ADR 0946, amended)",
    file: "src/lower/mir/tidy/logicals/chains.rs",
    find: "        found || matches!(&then[k..], [next] if calls(next))\n",
    replace: "        found || !then.is_empty()\n",
    tests: ["test/corpus.test.ts", "-t", "^let_chain_else_inside_if.rs$"],
    snapshots: true,
  },
];
