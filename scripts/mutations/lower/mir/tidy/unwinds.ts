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
    find: "                handler.remove(handler.len() - 2);\n",
    replace: "",
    tests: panics,
  },
  {
    name: "scope-across-throwing-statement",
    breaks: "a run goes on past a statement that may throw outside any `try`",
    file: "src/lower/mir/tidy/unwinds.rs",
    find: "                _ => stmt.only_makes_values(),\n",
    replace: "                _ => true,\n",
    tests: panics,
  },
  {
    name: "scoped-drops-if-uncovered",
    breaks: "an `if` in a guard's scope ends its run, so each statement keeps a `try` and `catch` of the drop, not one `finally`",
    file: "src/lower/mir/tidy/unwinds.rs",
    find: "                StmtKind::If(test, a, b) => {\n                    !test.may_throw()\n",
    replace: "                StmtKind::If(test, a, b) => {\n                    false && !test.may_throw()\n",
    tests: ["test/lowering-unwinds.test.ts", "-t", "a guard's scope is one finally"],
  },
  {
    name: "scoped-drops-exit-kept",
    breaks: "a way out of a guard's scope keeps its drop in the `try`, which the `finally` runs again",
    file: "src/lower/mir/tidy/unwinds.rs",
    find: "            out.push(leaving);\n            continue;\n",
    replace: "            out.push(stmt);\n            out.push(leaving);\n            continue;\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_guard_scope_returns.rs$"],
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
  {
    name: "result-not-returned-in-try",
    breaks: "`let result; try { result = e; } finally { .. } return result;` stays, where it's `try { return e; } finally { .. }`",
    file: "src/lower/mir/tidy/unwinds.rs",
    find: "        if found {\n            stmts.remove(i + 2);\n",
    replace: "        if found && false {\n            stmts.remove(i + 2);\n",
    tests: ["test/lowering.test.ts", "-t", "a Drop dropped once"],
  },
  {
    name: "scoped-drops-unread-in-place",
    breaks: "what a statement's `try` held apart stays apart once its scope is one `finally`, `const tmp = lines[s].includes(..); if (tmp)`",
    file: "src/lower/mir/tidy/unwinds.rs",
    find: "            stmts.extend(rest);\n            scoped = true;\n",
    replace: "            stmts.extend(rest);\n",
    tests: ["test/lowering-unwinds.test.ts", "-t", "a guard's fields are its variables"],
  },
];
