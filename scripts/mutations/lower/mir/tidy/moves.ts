// Mutations of src/lower/mir/tidy/moves.rs (ADR 0093): a temporary's
// value moved where what the program does would change.
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "fixed-place-holder-kept",
    breaks:
      "what holds a place MIR's lowering fixed stays `const item = $unwrap(resource.state); return (item.loading & 4) === 0;`, an `&&` split around it, not read where it's read once (rd202)",
    file: "src/lower/mir/tidy/moves.rs",
    find: "    if !is_temporary(name) && !element && !facts.fixed.contains(name) {\n",
    replace: "    if !is_temporary(name) && !element {\n",
    tests: ["test/corpus.test.ts", "-t", "^(cell_reached_through_a_call|unwrapped_cell_in_a_chain).rs$"],
    snapshots: true,
  },
  {
    name: "call-moved-past-a-getters-read",
    breaks:
      "a call's value is read past a `const` of a getter's read made after it, `const unknown = props.href == null ..; if (attr(\"href\") === unknown)`, so the getter runs first and the call sees what it did (rd178)",
    file: "src/lower/mir/tidy/moves.rs",
    find: "        .all(|s| matches!(&s.kind, StmtKind::Const(_, v) if passes(v) && !(acts && v.has_effects())))\n",
    replace: "        .all(|s| matches!(&s.kind, StmtKind::Const(_, v) if passes(v)))\n",
    tests: ["test/lowering-moves.test.ts", "-t", "a call isn't read past a getter's read"],
  },
  {
    name: "try-not-entered",
    breaks:
      "a value that can't throw made before a guard's `try` stays held there, `const focus = element.focus; try { (focus || ..).call(..) }`, not read in the `try` where its call is (rd233)",
    file: "src/lower/mir/tidy/moves.rs",
    find: "            if !value.may_throw() && js::mentions_in(after, name) == 0 && js::mentions_in(caught, name) == 0 =>\n",
    replace: "            if false =>\n",
    tests: [
      "test/lowering-is-function.test.ts",
      "-t",
      "js::method of a path reads",
    ],
  },
  {
    name: "try-entered-of-what-throws",
    breaks:
      "a value that may throw, made before a guard exists, is read in the guard's `try`, so its panic drops the guard it ran before (rd233)",
    file: "src/lower/mir/tidy/moves.rs",
    find: "            if !value.may_throw() && js::mentions_in(after, name) == 0 && js::mentions_in(caught, name) == 0 =>\n",
    replace:
      "            if js::mentions_in(after, name) == 0 && js::mentions_in(caught, name) == 0 =>\n",
    tests: ["test/corpus.test.ts", "-t", "^value_read_in_a_guards_try.rs$"],
  },
  {
    name: "try-entered-read-in-catch",
    breaks:
      "a value its `try`'s `catch` reads too is read only in the `try`, so the `catch` reads a variable nothing declares (rd233)",
    file: "src/lower/mir/tidy/moves.rs",
    find: "            if !value.may_throw() && js::mentions_in(after, name) == 0 && js::mentions_in(caught, name) == 0 =>\n",
    replace:
      "            if !value.may_throw() && js::mentions_in(after, name) == 0 =>\n",
    tests: ["test/corpus.test.ts", "-t", "^value_read_in_a_guards_try.rs$"],
  },
  {
    name: "stable-value-held-past-target",
    breaks:
      "a closure given to a setter under a drop guard stays a temporary, `function value() { .. } channel.port1.onmessage = value;`, as the setter's target reads a getter first (#393)",
    file: "src/lower/mir/tidy/moves.rs",
    find: "        StmtKind::Assign(target, _) if stable || !target.has_effects() => {\n",
    replace:
      "        StmtKind::Assign(target, _) if !target.has_effects() => {\n",
    tests: ["test/lowering-moves.test.ts", "-t", "under a drop guard"],
  },
  {
    name: "ended-borrow-by-js-names",
    breaks:
      "a `let` that ends a borrow is read in place only where its JS name is the property's, so `const return$ = node.return;` and `const debugHookTypes = current._debugHookTypes;` stay, though each is named as its field in Rust (ADR 0866, amended)",
    file: "src/lower/mir/tidy/moves.rs",
    find: "            && matches!(&value.kind, ExprKind::Member(..))\n            && place_read(value, name)\n",
    replace:
      "            && matches!(&value.kind, ExprKind::Member(_, property) if property == name)\n            && place_read(value, name)\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_let_named_as_its_field.rs$"],
    snapshots: true,
  },
  {
    name: "tested-field-held",
    breaks:
      "a field an or-pattern tests is held first, `const status = payload.status; status === 0 || status === -1`, not read in each test (ADR 0780)",
    file: "src/lower/mir/tidy/moves.rs",
    find: "    if !single && (assigned || !tested_in_place(value, expr, name, js::mentions_in(rest, name))) {\n",
    replace: "    if !single {\n",
    tests: [
      "test/lowering-or-patterns.test.ts",
      "-t",
      "tested in the order written",
    ],
  },
  {
    name: "tested-getter-in-place",
    breaks:
      "a getter's value an or-pattern tests is read in each test, the getter called again for each (ADR 0780)",
    file: "src/lower/mir/tidy/moves.rs",
    find: "        if place.facts.has(js::Known::Getter) {\n",
    replace: "        if false {\n",
    tests: [
      "test/lowering-or-patterns.test.ts",
      "-t",
      "getter's value reads it once",
    ],
  },
  {
    name: "mir-tidy-captured-kept",
    breaks:
      "what an element made before a branch reads stays a `const className = cn(…)`, not read where it's written",
    file: "src/lower/mir/tidy/moves.rs",
    find: "    if !is_temporary(name) && !element && !facts.fixed.contains(name) {\n",
    replace: "    if !is_temporary(name) && !facts.fixed.contains(name) {\n",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "mir-tidy-element-read-held",
    breaks:
      "what an element made ahead of a branch reads stays a `const` where elements before it come first, `className$1`",
    file: "src/lower/mir/tidy/moves.rs",
    find: "        true if element => {\n",
    replace: "        true if false && element => {\n",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "mir-tidy-inlined-no-backtrack",
    breaks:
      "of temporaries one after another, only the last is read where it's read, `cn(_8, _14, x)`",
    file: "src/lower/mir/tidy/moves.rs",
    find: "        // The one before it may now be read by the statement after it too.\n        i = i.saturating_sub(1);\n",
    replace: "",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "mir-tidy-inlined-not-past",
    breaks:
      "a temporary is read in place only by the statement right after it, `const _34 = formatted; const args$1 = args;` kept",
    file: "src/lower/mir/tidy/moves.rs",
    find: "    if !past {\n",
    replace: "    if !past || at > 0 {\n",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "target-root-not-inlined",
    breaks:
      "`const _25 = ref.current; _25[k] = v` stays, not `ref.current[k] = v` (and no `??=` of it)",
    file: "src/lower/mir/tidy/moves.rs",
    find: "                (true, true) => Some(Read::Target),\n",
    replace: "                (true, true) => None,\n",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "mir-tidy-past-borrow-blocks",
    breaks:
      "a chain stays a `const _4` ahead of a callback's borrow, `const list$1 = list;`, not the array's own `forEach`",
    file: "src/lower/mir/tidy/moves.rs",
    find: "    if !past {\n",
    replace: "    if !past || (acts && at > 0) {\n",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "mir-tidy-guarded-read-blocked",
    breaks:
      "a value made where its test holds isn't read where that test holds, `const className$2 = error != null ? cn(..) : undefined` kept",
    file: "src/lower/mir/tidy/moves.rs",
    find: "            if guard.is_some_and(|g| same_test(a, g))",
    replace: "            if false && guard.is_some_and(|g| same_test(a, g))",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "mir-pure-value-held-past-calls",
    breaks:
      "what does nothing, of a `let` no closure sets, `showToc && toc.length !== 0`, is held ahead of the calls before its read",
    file: "src/lower/mir/tidy/moves.rs",
    find: "unchanged_by_calls || !e.has_effects()",
    replace: "!e.has_effects()",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "inlined-pure-not-past-statements",
    breaks:
      'what does nothing, of variables no statement between assigns, `version === "Canary"`, stays held before a loop',
    file: "src/lower/mir/tidy/moves.rs",
    find: "\n        || (unchanged_by_calls && reads.iter().all(|v| !assigns(&rest[..at], v)));",
    replace: ";",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "jsx-key-read-in-written-order",
    breaks:
      "a `key` that does something is read in place, made after the children by JS's JSX runtime, `[1, 3, 2]`",
    file: "src/lower/mir/tidy/moves.rs",
    find: "                && let Some(found) = leads(key, name, guard, passes)\n",
    replace:
      "                && false\n                && let Some(found) = leads(key, name, guard, passes)\n",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "closure-of-statements-called-in-place",
    breaks:
      "a closure of statements is moved into its call, `x ? ((l) => { .. })(x) : undefined`, not called by its name",
    file: "src/lower/mir/tidy/moves.rs",
    find: "                left_called |= applied_in_place(value, args).is_none();\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(adapter_drops|tuple_scrutinee_calls).rs$",
    ],
    snapshots: true,
  },
  {
    name: "inlined-not-past-a-method",
    breaks:
      "a call's value MIR holds before a branch stays a `const value$1 = ..`, not read in `out.push(..)`'s arguments past the method",
    file: "src/lower/mir/tidy/moves.rs",
    find: "        ExprKind::Call(f, args) if let ExprKind::Member(receiver, _) = &f.kind => {\n",
    replace:
      "        ExprKind::Call(f, args) if false && let ExprKind::Member(receiver, _) = &f.kind => {\n",
    tests: ["test/lowering-moves.test.ts", "-t", "past the method"],
  },
  {
    name: "inlined-not-into-blocks",
    breaks:
      "a test MIR holds before the blocks its arms leave stays a `const value = ..`, not read where it's tested",
    file: "src/lower/mir/tidy/moves.rs",
    find: "        StmtKind::Labeled(_, body) if js::mentions_in(after, name) == 0 => entered(stmt, body, facts),\n",
    replace:
      "        StmtKind::Labeled(_, body) if false && js::mentions_in(after, name) == 0 => entered(stmt, body, facts),\n",
    tests: ["test/lowering-moves.test.ts", "-t", "block after it"],
  },
  {
    name: "binding-method-lookup-passed",
    breaks:
      "a call's value is read past a binding's method of a JS value, which a getter may answer, looked up before it runs",
    file: "src/lower/mir/tidy/moves.rs",
    find: "                && f.facts.has(js::Known::Getter) =>\n",
    replace: "                && false =>\n",
    tests: [
      "test/lowering-moves.test.ts",
      "-t",
      "a binding's method is looked up after its arguments run",
    ],
  },
  {
    name: "target-root-acting-held",
    breaks:
      "a temporary that does something, an assignment target's object, is held, `const document = window.document; document.title = t`",
    file: "src/lower/mir/tidy/moves.rs",
    find: "            let set_of = matches!(&root.kind, ExprKind::Var(v) if v == name);\n",
    replace:
      "            let set_of = matches!(&root.kind, ExprKind::Var(v) if v == name) && false;\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^returned_references.rs$"],
  },
  {
    name: "made-cell-read-unfolded",
    breaks:
      "an optional field's cell made in its temporary's place is read `{}.value` where its struct is made, a key that's there",
    file: "src/lower/mir/tidy/moves.rs",
    find: "    if read != Read::Target {\n",
    replace: "    if false {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(cell_const|ref_cell_field_const).rs$",
    ],
    snapshots: true,
  },
  {
    name: "made-cell-target-folded",
    breaks:
      "an assignment to a cell made in its temporary's place, `{ value: x }.value = v`, is `x = v`",
    file: "src/lower/mir/tidy/moves.rs",
    find: "    if read != Read::Target {\n",
    replace: "    if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^temporary_borrows.rs$"],
  },
  {
    name: "ended-borrow-kept",
    breaks:
      "a `let` that ends a borrow, read once by the store right after it, stays a `const`, `const child = current.child; workInProgress.child = child;`",
    file: "src/lower/mir/tidy/moves.rs",
    find: "            && ends_borrow.contains(name)\n",
    replace: "            && ends_borrow.contains(name) && false\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_let_read_in_place.rs$"],
    snapshots: true,
  },
  {
    name: "ended-borrow-passed",
    breaks:
      "a `let` that ends a borrow, read by the statement after it as an argument, `const sibling = node.sibling; log.push(.. nameOf(sibling))`, loses the program's name, where upstream's `handlerOnStack` keeps it",
    file: "src/lower/mir/tidy/moves.rs",
    find: "    let StmtKind::Assign(_, value) = &stmt.kind else {\n        return false;\n    };\n",
    replace:
      "    let StmtKind::Assign(_, value) = &stmt.kind else {\n        return true;\n    };\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_let_read_in_place.rs$"],
    snapshots: true,
  },
  {
    name: "ended-borrow-nulled-kept",
    breaks:
      "a `let` that ends a borrow, stored in a nullable field, `workInProgress.child = child ?? null`, stays a `const`",
    file: "src/lower/mir/tidy/moves.rs",
    find: "        ExprKind::Binary(Op::Coalesce, value, null) if matches!(null.kind, ExprKind::Null) => value,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^refcell_let_read_in_place.rs$"],
    snapshots: true,
  },
  {
    name: "tested-read-of-a-written-let",
    breaks:
      "a `let` the program wrote, `let types = t.types.borrow().clone(); if let Some(types) = types`, loses its name to its place's read, `if (t.types != null) { return t.types.length; }` (rd226, the user's decision)",
    file: "src/lower/mir/tidy/moves.rs",
    find: "    if !own_name || named.contains(name) {\n",
    replace: "    if !own_name {\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_let_taken_apart.rs$"],
    snapshots: true,
  },
  {
    name: "tested-read-past-a-call",
    breaks:
      "a pattern's binding is read in place where the test runs code first, `checked(named.autoName)`, which may change what it reads",
    file: "src/lower/mir/tidy/moves.rs",
    find: "    if !place || tested == 0 || test.has_effects() {\n",
    replace: "    if !place || tested == 0 {\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_let_taken_apart.rs$"],
    snapshots: true,
  },
  {
    name: "read-again-twice",
    breaks:
      "a let chain's binding read twice in the body is the field read twice, the second after the first's call, which may change it (ADR 0866, amended)",
    file: "src/lower/mir/tidy/moves.rs",
    find: "    if js::mentions_in(&stmts[at + 1..], name) != 1 {\n",
    replace: "    if js::mentions_in(&stmts[at + 1..], name) == 0 {\n",
    tests: [
      "test/lowering-written-branches.test.ts",
      "-t",
      "binding of a field the test read is read again",
    ],
  },
  {
    name: "read-again-past-code",
    breaks:
      "a let chain's binding read after code its statement runs first, `probe(f) + task.run(..)`, is the field read there, after `probe` changed it (ADR 0866, amended)",
    file: "src/lower/mir/tidy/moves.rs",
    find: "    if leads(read.of(next), name, None, &|e| !e.has_effects()) != Some(true) {\n",
    replace:
      "    if leads(read.of(next), name, None, &|_| true) != Some(true) {\n",
    tests: [
      "test/lowering-written-branches.test.ts",
      "-t",
      "binding of a field the test read is read again",
    ],
  },
  {
    name: "read-again-of-a-later-statement",
    breaks:
      "a let chain's binding read in a later statement, past one that changes the field, is the field read there (ADR 0866, amended)",
    file: "src/lower/mir/tidy/moves.rs",
    find: "    if leads(read.of(next), name, None, &|e| !e.has_effects()) != Some(true) {\n",
    replace:
      "    if leads(read.of(next), name, None, &|e| !e.has_effects()) == Some(false) {\n",
    tests: [
      "test/lowering-written-branches.test.ts",
      "-t",
      "binding of a field the test read is read again",
    ],
  },
  {
    name: "own-name-read-never-in-place",
    breaks:
      "React's getViewTransitionName keeps `const autoName = instance.autoName;`, the field read where upstream reads it in place",
    file: "src/lower/mir/tidy/moves.rs",
    find: "    if !own_name || named.contains(name) {\n",
    replace: "    if true || named.contains(name) {\n",
    tests: [
      "test/lowering-written-branches.test.ts",
      "-t",
      "an if let of a field by its own name",
    ],
  },
  {
    name: "object-spread-read-before-call",
    breaks:
      '`js::object::spread((base, dict::from_entries(vec![("c", counted(base))])))` reads `base` before `counted(base)` sets it, `{ ...base, ...Object.fromEntries([["c", counted(base)]]) }`, where Rust reads it after (ADR 0967, amended)',
    file: "src/lower/mir/tidy/moves.rs",
    find: "                    js::Prop::Spread(v) => std::borrow::Cow::Owned(Expr::spread(v.clone())),\n",
    replace:
      "                    js::Prop::Spread(v) => std::borrow::Cow::Borrowed(v),\n",
    tests: [
      "test/js-collections.test.ts",
      "-t",
      "a Dict spread before more of it",
    ],
  },
  {
    name: "held-receivers-off",
    breaks:
      "a field's function called with what needs statements is called from a variable, `const L = internals.d.L; ..; L(href, s)`, with no `this` (rd285)",
    file: "src/lower/mir/tidy/moves.rs",
    find: "        if !is_temporary(name) || !value.facts.has(js::Known::Lasting) || js::mentions_in(&stmts[i + 1..], name) != 1 {\n",
    replace: "        if true {\n",
    tests: ["test/lowering-held-receivers.test.ts"],
  },
  {
    name: "held-receivers-of-any-field",
    breaks:
      "a field Rust changes between is read after what's given, so `(inner.f)({ inner.f = triple; n })` calls `triple` (rd285)",
    file: "src/lower/mir/tidy/moves.rs",
    find: "        if !is_temporary(name) || !value.facts.has(js::Known::Lasting) || js::mentions_in(&stmts[i + 1..], name) != 1 {\n",
    replace:
      "        if !is_temporary(name) || js::mentions_in(&stmts[i + 1..], name) != 1 {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^field_functions_called_after_arguments.rs$",
    ],
  },
];
