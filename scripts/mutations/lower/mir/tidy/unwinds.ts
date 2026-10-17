// Mutations of src/lower/mir/tidy/unwinds.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

const panics = ["test/corpus.test.ts", "-t", "^(drop_on_panic|question_drops|vec_destructors|dyn_auto_traits|scoped_drops_cross).rs$"];

export const mutations: Mutation[] = [
  {
    name: "scoped-drops-kept-per-statement",
    breaks: "each statement a value's drop ends stays its own `try { .. } catch (error) { drop(x); throw error; }`, not one `try { .. } finally { drop(x) }`",
    file: "src/lower/mir/tidy/unwinds.rs",
    find: "        if let Some(end) = scope_end(stmts, i) {\n",
    replace: "        if let Some(end) = scope_end(stmts, i).filter(|_| false) {\n",
    tests: panics,
    snapshots: true,
  },
  {
    name: "scoped-drop-left-in-catch",
    breaks: "a statement's `catch` in a scope keeps the scope's drop, which a panic then runs twice, there and in the `finally`",
    file: "src/lower/mir/tidy/unwinds.rs",
    find: "                        handler.remove(handler.len() - 2);\n",
    replace: "",
    tests: panics,
  },
  {
    name: "scope-across-throwing-statement",
    breaks: "a run goes on past a statement that may throw outside any `try`",
    file: "src/lower/mir/tidy/unwinds.rs",
    find: "            _ if !made_of_values(stmt) => return None,\n",
    replace: "",
    tests: panics,
  },
  {
    name: "scope-declarations-unhoisted",
    breaks: "what a scope declares and what follows it reads is declared inside its `try`, out of reach",
    file: "src/lower/mir/tidy/unwinds.rs",
    find: "            let hoisted = hoist_read_after(&mut body, &rest);\n",
    replace: "            let hoisted: Vec<Stmt> = Vec::new();\n",
    tests: ["test/corpus.test.ts", "-t", "^(drop_scopes|drop_temporaries|scope_temporaries|option_combinator_drops|slice_pointers|subscriptions|lock_destructors|drop_generic|dyn_drops).rs$"],
  },
  {
    name: "scope-lets-left-before",
    breaks: "a `let x;` only a scope reads stays before its `try`, `let tmp; try { tmp = .. }`",
    file: "src/lower/mir/tidy/unwinds.rs",
    find: "                && js::mentions_in(&rest, name) == 0\n",
    replace: "                && false\n",
    tests: panics,
    snapshots: true,
  },
];
