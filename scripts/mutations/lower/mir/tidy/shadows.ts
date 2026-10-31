// Mutations of src/lower/mir/tidy/shadows.rs (ADR 0093): a shadow made
// the binding it shadows where something may still read that one, or kept
// apart where nothing does (ADRs 0609, 0650).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "shadow-set-in-a-branch-that-does-more-apart",
    breaks:
      "a shadow set in an `if` branch that does more than give the binding stays apart, React's createWorkInProgress `const workInProgress = ..; let workInProgress$1;` (ADR 0650, amended)",
    file: "src/lower/mir/tidy/shadows.rs",
    find: "            if list.len() > here {\n",
    replace: "            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^shadow_set_in_cases.rs$"],
    snapshots: true,
  },
  {
    name: "shadow-set-in-a-branch-of-it-alone",
    breaks:
      "a shadow whose `if` branch is the binding's own value alone is the binding, the branch left empty (ADR 0650, amended)",
    file: "src/lower/mir/tidy/shadows.rs",
    find: "            if list.len() > here {\n",
    replace: "            if list.len() >= here {\n",
    tests: ["test/corpus.test.ts", "-t", "^shadow_set_in_cases.rs$"],
    snapshots: true,
  },
  {
    name: "shadow-kept-apart",
    breaks:
      "a `let` that shadows a binding read no more is `const stack$1`, not `stack = ..`",
    file: "src/lower/mir/tidy/shadows.rs",
    find: "    let mut captured = captured(body);\n    shadowed_block(body, &heads, user, &mut captured);\n",
    replace: "    let _ = (heads, user, captured(body));\n",
    tests: ["test/lowering-shadows.test.ts", "-t", "React's formatOwnerStack"],
  },
  {
    name: "shadow-of-itself-assigned",
    breaks: "`let mut stack = stack.as_str();` is `stack = stack;`",
    file: "src/lower/mir/tidy/shadows.rs",
    find: "    let assigned = !matches!(&value.kind, ExprKind::Var(v) if *v == binding);\n",
    replace: "    let assigned = true;\n",
    tests: ["test/lowering-shadows.test.ts", "-t", "React's formatOwnerStack"],
  },
  {
    name: "shadow-binding-kept-const",
    breaks:
      "the binding a shadow assigns stays a `const`, and assigning it throws",
    file: "src/lower/mir/tidy/shadows.rs",
    find: "                Some(at) => set_binding(&mut stmts[at]),\n",
    replace: "                Some(_) => {}\n",
    tests: ["test/lowering-shadows.test.ts", "-t", "React's formatOwnerStack"],
  },
  {
    name: "shadow-for-pattern-kept-const",
    breaks:
      "a `for`'s pattern a shadow assigns stays `const`, and assigning it throws",
    file: "src/lower/mir/tidy/shadows.rs",
    find: "                if shadowed_block(body, &names, user, captured) {\n                    *mutable = true;\n                }\n",
    replace: "                shadowed_block(body, &names, user, captured);\n",
    tests: ["test/lowering-shadows.test.ts", "-t", "something still sees"],
  },
  {
    name: "shadow-of-what-is-read-after",
    breaks:
      "a binding read after its shadow, by a drop at the scope's end say, is assigned the shadow's value first",
    file: "src/lower/mir/tidy/shadows.rs",
    find: "    if js::mentions_in(&stmts[i + 1..], &binding) > 0 {\n        return None;\n    }\n",
    replace: "",
    tests: ["test/lowering-shadows.test.ts", "-t", "something still sees"],
  },
  {
    name: "shadow-of-what-a-closure-reads",
    breaks:
      "a binding a closure made before reads is assigned, so the closure sees the new value",
    file: "src/lower/mir/tidy/shadows.rs",
    find: "    if !user.contains(&shadow) || !user.contains(&binding) || captured.contains(&binding) {\n",
    replace: "    if !user.contains(&shadow) || !user.contains(&binding) {\n",
    tests: ["test/lowering-shadows.test.ts", "-t", "something still sees"],
  },
  {
    name: "shadow-across-a-loop",
    breaks:
      "a `let` in a loop shadows a parameter outside it, which the next turn reads anew, and reads the last turn's value",
    file: "src/lower/mir/tidy/shadows.rs",
    find: "            StmtKind::While { body, .. } | StmtKind::For { body, .. } | StmtKind::Labeled(_, body) => {\n                shadowed_block(body, &[], user, captured);\n",
    replace:
      "            StmtKind::While { body, .. } | StmtKind::For { body, .. } | StmtKind::Labeled(_, body) => {\n                shadowed_block(body, heads, user, captured);\n",
    tests: ["test/lowering-shadows.test.ts", "-t", "something still sees"],
  },
  {
    name: "shadow-of-a-temporary",
    breaks:
      "a temporary rust-js holds, `tmp`, is reused for unrelated values, `tmp = tmp(next)`",
    file: "src/lower/mir/tidy/shadows.rs",
    find: "    if !user.contains(&shadow) || !user.contains(&binding) || captured.contains(&binding) {\n",
    replace: "    if captured.contains(&binding) {\n",
    tests: ["test/corpus.test.ts", "-t", "^iter_next_back.rs$"],
    snapshots: true,
  },
  {
    name: "shadow-set-in-cases-kept-apart",
    breaks:
      "a `let x$1;` its `switch`'s cases set, shadowing a binding read no more, stays apart, `let priorityLevel$1; switch (priorityLevel) { .. priorityLevel$1 = priorityLevel; ..`, not upstream's `switch (priorityLevel) { .. default: priorityLevel = NormalPriority; }`",
    file: "src/lower/mir/tidy/shadows.rs",
    find: "        set_after_last_read(&stmts[i + 1..], &shadow, &binding, false)?;\n",
    replace:
      "        set_after_last_read(&stmts[i + 1..], &shadow, &binding, false).filter(|_| false)?;\n",
    tests: ["test/port-scheduler.test.ts"],
  },
  {
    name: "shadow-set-in-cases-own-value-kept",
    breaks:
      "a case of a shadow's own value is `priorityLevel = priorityLevel;`, not the `break;` alone upstream's is",
    file: "src/lower/mir/tidy/shadows.rs",
    find: "        js::statement_lists(&mut rest, &mut |list| list.retain(|s| !own(s)));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^shadow_set_in_cases.rs$"],
    snapshots: true,
  },
  {
    name: "shadow-set-in-branches-own-value-emptied",
    breaks:
      "a shadow of an `if` chain one of whose branches is the binding's own value is the binding, that branch left empty, `if (level >= 1 && level <= 3) {} else ..`",
    file: "src/lower/mir/tidy/shadows.rs",
    find: "        if owns != kept {\n",
    replace: "        if false && owns != kept {\n",
    tests: ["test/corpus.test.ts", "-t", "^shadow_set_in_cases.rs$"],
    snapshots: true,
  },
  {
    name: "shadow-set-in-cases-read-after",
    breaks:
      "a `let noisy$1;` set in a `switch`'s cases is the binding it shadows though a drop reads that after, which then drops the new value twice",
    file: "src/lower/mir/tidy/shadows.rs",
    find: "        if set && js::mentions_in(std::slice::from_ref(stmt), old) > 0 {\n",
    replace:
      "        if false && set && js::mentions_in(std::slice::from_ref(stmt), old) > 0 {\n",
    tests: ["test/corpus.test.ts", "-t", "^shadow_set_in_cases.rs$"],
  },
  {
    name: "shadow-set-in-cases-fallen-unset",
    breaks:
      "a guarded case that sets the shadow and leaves is taken to fall through set, so the next case's read of the old binding keeps the shadow apart",
    file: "src/lower/mir/tidy/shadows.rs",
    find: "                    fallen = case.falls && on(&case.body, end);\n",
    replace: "                    fallen = case.falls;\n",
    tests: ["test/corpus.test.ts", "-t", "^shadow_set_in_cases.rs$"],
    snapshots: true,
  },
  {
    name: "shadow-set-in-cases-left-branch-goes-on",
    breaks:
      "a branch that sets the shadow and leaves, `if (strict) { level$1 = ..; break; }`, is taken to go on set",
    file: "src/lower/mir/tidy/shadows.rs",
    find: "        let on = |stmts: &[Stmt], (on, _): (bool, bool)| on && !stmts.last().is_some_and(js::ends);\n",
    replace: "        let on = |_: &[Stmt], (on, _): (bool, bool)| on;\n",
    tests: ["test/corpus.test.ts", "-t", "^shadow_set_in_cases.rs$"],
    snapshots: true,
  },
  {
    name: "shadow-set-in-cases-unnamed-rest-followed",
    breaks:
      "what follows a shadow's `switch` and names the old binding nowhere is followed still, a loop or a `try` there keeping the shadow apart",
    file: "src/lower/mir/tidy/shadows.rs",
    find: "        if js::mentions_in(&stmts[k..], old) == 0 {\n",
    replace: "        if false && js::mentions_in(&stmts[k..], old) == 0 {\n",
    tests: ["test/corpus.test.ts", "-t", "^shadow_set_in_cases.rs$"],
    snapshots: true,
  },
  {
    name: "shadow-of-an-if-branch-kept",
    breaks:
      "a shadow one `if` branch of which is the binding's own value alone, `Some(list) => list`, keeps its `$1`, `let list$1; if (list != null) { list$1 = list; } else { .. }`, where upstream's addEventHandleToTarget sets one variable (rd65)",
    file: "src/lower/mir/tidy/shadows.rs",
    find: "                    kept += usize::from(of_binding(true) && alone(then) && !otherwise.is_empty() && !alone(otherwise));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^shadow_made_where_none.rs$"],
    snapshots: true,
  },
  {
    name: "shadow-emptied-if-unflipped",
    breaks:
      "an `if` whose branch the shadow's own value left empty stays `if (list != null) {} else { .. }`, not its other way, `if (list == null) { .. }` (rd65)",
    file: "src/lower/mir/tidy/shadows.rs",
    find: "                        *test = std::mem::replace(test, Expr::undefined()).negated_test();\n                        *then = otherwise.take().unwrap_or_default();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^shadow_made_where_none.rs$"],
    snapshots: true,
  },
];
