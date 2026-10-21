// Mutations of src/lower/mir/tidy/bindings.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "tidy-effectful-init-dropped",
    breaks: "`let a = made(1); a = 2;` is `let a = 2;`, and `made(1)` never runs",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "is_none_or(|e| !e.has_effects())",
    replace: "is_none_or(|e| true || !e.has_effects())",
    tests: ["test/corpus.test.ts","-t","declarations_assigned_later"],
  },
  {
    name: "tidy-declaration-past-read",
    breaks: "a `let` moves to its next assignment past a read of it, which reads it before it's declared",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "                    .find(|&j| js::mentions_in(&stmts[j..=j], name) > 0)\n",
    replace: "                    .find(|&j| matches!(stmts[j].kind, StmtKind::Assign(..)) && js::mentions_in(&stmts[j..=j], name) > 0)\n",
    tests: ["test/corpus.test.ts","-t","declarations_assigned_later"],
  },
  {
    name: "tidy-closure-assign-unseen",
    breaks: "a variable assigned again, a parameter, is taken for one nothing changes: a tuple's part of it is read after a guard set it",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "            {\n                assigned.insert(name.clone());\n            }\n        }\n",
    replace: "            {\n                let _ = name;\n            }\n        }\n",
    tests: ["test/corpus.test.ts", "-t", "tuple_scrutinee_calls|declarations_assigned_later"],
  },
  {
    name: "mir-tidy-alias-kept",
    breaks: "a `const` read once, by another `const` of it, stays a name for a name",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "                            && js::mentions_in(&stmts[j..], a) == 1\n",
    replace: "                            && false\n",
    tests: ["test/lowering.test.ts","-t","stepped by hand"],
  },
  {
    name: "mir-tidy-alias-variables-kept",
    breaks: "`const _5 = TName;` stays, not `TName` where `_5` is read",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "                if !kept {\n",
    replace: "                if false {\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-tidy-named-temporary-kept-binding",
    breaks: "the `const found = _4;` a temporary's renaming took stays, `const found = found;`",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "    stmts.retain(|s| {\n        !matches!(&s.kind, StmtKind::Const(n, value)\n            if n == b && matches!(&value.kind, ExprKind::Var(v) if v == a))\n    });\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","if-let's binding from MIR keeps"],
  },
  {
    name: "mir-tidy-captured-kept",
    breaks: "what an element made before a branch reads stays a `const className = cn(…)`, not read where it's written",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "        let temporary = is_temporary(name) || captured.contains(name);\n",
    replace: "        let temporary = is_temporary(name);\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-tidy-let-not-into-branch",
    breaks: "a `let` one branch alone names is declared before the `if`, not in the branch",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "            if let Some(branch) = branch {\n",
    replace: "            if let Some(branch) = branch.filter(|_| false) {\n",
    tests: ["test/lowering.test.ts","-t","declared where it.s first named"],
  },
  {
    name: "mir-tidy-element-read-held",
    breaks: "what an element made ahead of a branch reads stays a `const` where elements before it come first, `className$1`",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "            true if element => {\n",
    replace: "            true if false && element => {\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-tidy-inlined-no-backtrack",
    breaks: "of temporaries one after another, only the last is read where it's read, `cn(_8, _14, x)`",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "        i = i.saturating_sub(1);\n",
    replace: "",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-tidy-inlined-not-past",
    breaks: "a temporary is read in place only by the statement right after it, `const _34 = formatted; const args$1 = args;` kept",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "        let past = |at: usize| {\n",
    replace: "        let past = |at: usize| at == 0 && {\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-tidy-loop-let-outside",
    breaks: "a `let` a loop's body assigns before it's read is declared outside the loop, `let start;`",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "            && !looped_reads(&stmts[first], &name)\n",
    replace: "            && false\n",
    tests: ["test/lowering.test.ts","-t","loop.s body makes, from MIR"],
  },
  {
    name: "mir-tidy-let-outside-label",
    breaks: "a `let` only a labeled block names stays before it, `let _7;`",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "        if alone && let StmtKind::Labeled(_, body) = &mut stmts[first].kind {\n",
    replace: "        if false && alone && let StmtKind::Labeled(_, body) = &mut stmts[first].kind {\n",
    tests: ["test/lowering.test.ts","-t","of .matches!. from MIR"],
  },
  {
    "name": "target-root-not-inlined",
    "breaks": "`const _25 = ref.current; _25[k] = v` stays, not `ref.current[k] = v` (and no `??=` of it)",
    "file": "src/lower/mir/tidy/bindings.rs",
    "find": "                    (true, true) => Some(target),\n",
    "replace": "                    (true, true) => None,\n",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-tidy-past-borrow-blocks",
    "breaks": "a chain stays a `const _4` ahead of a callback's borrow, `const list$1 = list;`, not the array's own `forEach`",
    "file": "src/lower/mir/tidy/bindings.rs",
    "find": "        let past = |at: usize| {\n",
    "replace": "        let past = |at: usize| (!acts || at == 0) && {\n",
    "tests": ["test/jsx.test.ts"]
  },
  {
    "name": "mir-tidy-guarded-read-blocked",
    "breaks": "a value made where its test holds isn't read where that test holds, `const className$2 = error != null ? cn(..) : undefined` kept",
    "file": "src/lower/mir/tidy/bindings.rs",
    "find": "            if guard.is_some_and(|g| same_test(a, g))",
    "replace": "            if false && guard.is_some_and(|g| same_test(a, g))",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-pure-value-held-past-calls",
    "breaks": "what does nothing, of a `let` no closure sets, `showToc && toc.length !== 0`, is held ahead of the calls before its read",
    "file": "src/lower/mir/tidy/bindings.rs",
    "find": "unchanged_by_calls || !e.has_effects()",
    "replace": "!e.has_effects()",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "loop-let-branch-assigned-not-sunk",
    "breaks": "a `let` a loop's branch makes from MIR stays one `let` before the loop, not a `const` in the branch",
    "file": "src/lower/mir/tidy/bindings.rs",
    "find": "                && (alone && assigned(then) && otherwise.as_deref().is_none_or(assigned)",
    "replace": "                && (false",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a let a loop's branch makes is declared in the loop"
    ]
  },
  {
    "name": "let-else-copy-kept",
    "breaks": "a `let`-`else`'s binding of a variable stays a copy, `const current$1 = current`",
    "file": "src/lower/mir/tidy/bindings.rs",
    "find": "            && then.last().is_some_and(ends)\n            && let StmtKind::Const(b, value) = &stmts[i + 1].kind",
    "replace": "            && then.last().is_some_and(ends)\n            && false\n            && let StmtKind::Const(b, value) = &stmts[i + 1].kind",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a let-else's binding of a variable is the variable"
    ]
  },
  {
    "name": "alias-takes-prop-name",
    "breaks": "a named variable moved into a held prop takes the prop's name, `const code = ..`, and is read away",
    "file": "src/lower/mir/tidy/bindings.rs",
    "find": "                if !is_temporary(a) && (captured.contains(&b) || is_temporary(&b)) {",
    "replace": "                if false {",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "forwarded-value-kept",
    "breaks": "a value a `try` makes in each branch stays a temporary's, `children = tmp` after it",
    "file": "src/lower/mir/tidy/bindings.rs",
    "find": "            && last_assigns(&stmts[i + 1..=i + 1], t)\n",
    "replace": "            && false\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a value a try makes in each branch is the variable's there"
    ]
  },
  {
    "name": "inlined-pure-not-past-statements",
    "breaks": "what does nothing, of variables no statement between assigns, `version === \"Canary\"`, stays held before a loop",
    "file": "src/lower/mir/tidy/bindings.rs",
    "find": "                || (unchanged_by_calls && reads.iter().all(|v| !assigns(&rest[..at], v)))\n",
    "replace": "",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "held-prop-named-value",
    "breaks": "what MIR holds for a prop is named `value`, not after the prop, `const href = made(\"h\")`",
    "file": "src/lower/mir/tidy/bindings.rs",
    "find": "                        (Some(prop), _) => prop.clone(),\n",
    "replace": "                        (Some(_), _) => \"value\".to_string(),\n",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "jsx-key-read-in-written-order",
    "breaks": "a `key` that does something is read in place, made after the children by JS's JSX runtime, `[1, 3, 2]`",
    "file": "src/lower/mir/tidy/bindings.rs",
    "find": "                && let Some(found) = leads(key, name, guard, passes)\n",
    "replace": "                && false\n                && let Some(found) = leads(key, name, guard, passes)\n",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "held-arg-named-value",
    "breaks": "an argument MIR holds, `format!`'s, is named `value`, not `arg`",
    "file": "src/lower/mir/tidy/bindings.rs",
    "find": "(None, _) if is_held_arg(name) => \"arg\".to_string(),",
    "replace": "(None, _) if is_held_arg(name) => \"value\".to_string(),",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "format!"
    ]
  },
  {
    "name": "alias-takes-temporary-name",
    "breaks": "a variable the program names, `const props = {..}`, aliased by a temporary, takes the temporary's place and is read away",
    "file": "src/lower/mir/tidy/bindings.rs",
    "find": "if !is_temporary(a) && (captured.contains(&b) || is_temporary(&b)) {",
    "replace": "if !is_temporary(a) && captured.contains(&b) {",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-tidy-turn-let-branches-unseen",
    "breaks": "`let y;` of `let y = match x { 0 => continue, .. }` is declared before the loop, as an `if`'s branches each assigning it don't count",
    "file": "src/lower/mir/tidy/bindings.rs",
    "find": "                    || assigned_on(then, name) && otherwise.as_deref().is_some_and(|o| assigned_on(o, name)))",
    "replace": "                    || false)",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a let each turn assigns"
    ]
  },
  {
    "name": "mir-tidy-turn-continue-goes-on",
    "breaks": "a branch that `continue`s is taken to go on unassigned, and `let y;` stays before the loop",
    "file": "src/lower/mir/tidy/bindings.rs",
    "find": "            StmtKind::Continue(_) | StmtKind::Return(_) | StmtKind::Throw(_) | StmtKind::Break(None)",
    "replace": "            StmtKind::Return(_) | StmtKind::Throw(_) | StmtKind::Break(None)",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a let each turn assigns"
    ]
  },
  {
    name: "mir-tidy-receiver-named-value",
    breaks: "MIR's trait object a method is called on is named `value`, not `receiver`",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "            given.entry(v.clone()).or_insert_with(|| \"receiver\".to_string());\n",
    replace: "",
    tests: ["test/traits.test.ts","-t","MIR dictionaries are explicit"],
  },
  {
    name: "mir-tidy-unread-call-kept",
    breaks: "MIR's call whose value nothing reads is `const n = items.pop()`",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "            && matches!(value.kind, ExprKind::Call(..))\n",
    replace: "            && false\n",
    tests: ["test/lowering.test.ts","-t","a block's const reuses a name where each read keeps its own"],
  },
  {
    name: "arms-binding-one-name-apart",
    breaks: "`n` that two arms bind, `n` and `n$1` until named in their scopes, stays `const n = length;` in each",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "                base(other) != base(b)\n",
    replace: "                *other != *b\n",
    tests: ["test/corpus.test.ts", "-t", "tuple_scrutinee_calls"],
    snapshots: true,
  },
  {
    name: "user-name-taken-for-temporary",
    breaks: "a local the program names `_5` is taken for a temporary, and named `value`",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "        true => format!(\"{name}_\"),\n",
    replace: "        true => name.to_string(),\n",
    tests: ["test/corpus.test.ts", "-t", "temporary_shaped_names"],
    snapshots: true,
  },
  {
    name: "mir-tidy-param-module-name-reused",
    breaks: "MIR's closure parameter takes a name of the module's",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "                if named.contains(base) || module.contains(base) {\n",
    replace: "                if named.contains(base) {\n",
    tests: ["test/link.test.ts", "-t", "module-load statements"],
  },
  {
    name: "alias-of-changing-set-between",
    breaks: "an alias of a variable set before it's read is the variable, `pair(n, { n += 1; n })` given the new `n` twice",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "                let kept = last.is_some_and(|last| assigned(&mut stmts[i + 1..=last].to_vec()).contains(of));\n",
    replace: "                let kept = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^alias_reads.rs$"],
  },
  {
    name: "lets-kept",
    breaks: "`let list = []` that nothing sets again stays a `let`",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "    js::constants(body);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^cell_deque_char_methods.rs$"],
    snapshots: true,
  },
  {
    name: "let-not-sunk-through-loop",
    breaks: "a `let` of an inner loop is declared before the outer one, `let score;`, assigned in the inner",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "            alone && !looped_reads(&stmts[at], name) && assigned_first(body, name)\n",
    replace: "            false\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_loop_jumps.rs$"],
    snapshots: true,
  },
  {
    name: "let-not-sunk-through-labeled-block",
    breaks: "a `let` first assigned in a labeled block is declared before the loop around it",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "        StmtKind::Labeled(_, body) => alone && assigned_first(body, name),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(nested_loop_jumps|range_values|or_bindings|interpreter).rs$"],
    snapshots: true,
  },
  {
    name: "paired-temporaries-unpaired",
    breaks: "a temporary assigned then read once each time stays a `let` assigned and read, `tmp$7 = make(8); tmp$8 = [tmp$6, tmp$7]` (ADR 0564)",
    file: "src/lower/mir/tidy/bindings.rs",
    find: "                        true => *pairs.entry(name.clone()).or_default() += 1,\n",
    replace: "                        true => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^temporaries_taken_apart.rs$"],
    snapshots: true,
  },
];
