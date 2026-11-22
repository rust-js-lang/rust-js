// Mutations of src/lower/mir/tidy.rs (ADR 0364).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "falsy-const-some-where-unanded",
    breaks: "a `const` of a `SomeWhere` read only as a truth isn't `t && v`",
    file: "src/lower/mir/tidy.rs",
    find: "                && let Some((test, then, _)) = value.some_where_parts()\n",
    replace: "                && let Some((test, then, _)) = value.some_where_parts().filter(|(_, _, there)| !there)\n",
    tests: ["test/jsx-children.test.ts", "test/jsx-evaluation-order.test.ts", "test/jsx-types.test.ts", "test/jsx-react-dev.test.ts", "test/jsx-props-read.test.ts", "test/bindings.test.ts", "test/react.test.ts", "test/lowering-option-truth.test.ts", "test/snapshots.test.ts", "test/pilot.test.ts"],
  },
  {
    name: "comments-before-unprinted-return-dropped",
    breaks:
      "a tail `None`'s comments where its `return;` of nothing goes are taken for a `return null;`'s, and move before the code above them (ADR 0862, amended)",
    file: "src/lower/mir/tidy.rs",
    find: "                        if matches!(v.kind, ExprKind::Undefined | ExprKind::GivenUndefined)) =>\n",
    replace: "                        if false) =>\n",
    tests: [
      "test/lowering-comments.test.ts",
      "-t",
      "a comment before a None given at a function's end",
    ],
  },
  {
    name: "comments-before-tail-return-kept",
    breaks:
      "a tail `None`'s comments, where a nullable function's `return null;` is printed, stay a block's of only comments, printed after it (ADR 0444)",
    file: "src/lower/mir/tidy.rs",
    find: "                    && next.span.lo == comments.span.lo\n",
    replace: "                    && false\n",
    tests: [
      "test/lowering-comments-ends.test.ts",
      "-t",
      "a comment before a tail None",
    ],
  },
  {
    name: "mir-tidy-falsy-temporary-kept",
    breaks:
      "a temporary given where what's falsy is skipped stays `test ? value : undefined`, not `test && value`",
    file: "src/lower/mir/tidy.rs",
    find: "                *value = Expr::bin(Op::And, test.tested(), then.clone());\n",
    replace: "",
    tests: ["test/bindings.test.ts", "-t", "skips what's falsy"],
  },
  {
    name: "mir-tidy-tested-kept",
    breaks:
      'a test a skips-falsy call reads alone keeps its `!!`, `(!!error || ready) && "b"`',
    file: "src/lower/mir/tidy.rs",
    find: "                *value = value.tested();\n",
    replace: "",
    tests: ["test/bindings.test.ts", "-t", "skips what's falsy"],
  },
  {
    name: "shown-child-conditional",
    breaks:
      "a child shown where a test holds is `shown ? hint : undefined`, not `shown && hint`",
    file: "src/lower/mir/tidy.rs",
    find: "                && matches!(none.kind, ExprKind::Undefined)\n            {\n                let test = match is_boolean(test, booleans) {",
    replace:
      "                && false\n            {\n                let test = match is_boolean(test, booleans) {",
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
    name: "late-not-unsimplified",
    breaks: '`!matches!(t, "pre")` read in place stays `!(t === "pre")`',
    file: "src/lower/mir/tidy.rs",
    find: "        let made = Expr::unary(*op, (**arg).clone());\n",
    replace:
      "        let made = Expr { kind: ExprKind::Unary(*op, arg.clone()), facts: js::Facts::NONE, span: e.span, held_as: js::HeldAs(None) };\n",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "negated matches, string methods",
    ],
  },
  {
    name: "mir-tidy-concise-handlers-kept",
    breaks:
      "a held handler of one call from MIR keeps its block, `() => { f(); }`",
    file: "src/lower/mir/tidy.rs",
    find: "                && of(value).is_some()\n",
    replace: "                && false\n",
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
    name: "jsx-child-truth-test-kept",
    breaks:
      "a child shown where text is truthy stays `href ? <a /> : undefined`, not `!!href && <a />`",
    file: "src/lower/mir/tidy.rs",
    find: "                    false => Expr::unary(UnaryOp::Not, Expr::unary(UnaryOp::Not, (**test).clone())),",
    replace: "                    false => continue,",
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
    name: "mir-tidy-return-span-none",
    breaks: "MIR's `return` of a value put in it maps to no Rust",
    file: "src/lower/mir/tidy.rs",
    find: "            StmtKind::Return(Some(value)) if stmt.span.is_none() => stmt.span = value.span,\n",
    replace: "",
    tests: ["test/emission.test.ts", "-t", "source map points from fib.js"],
  },
  {
    name: "mir-tidy-if-span-of-test-kept",
    breaks:
      "MIR's `if` maps to its condition or its `match`, not `if c` or its arm's pattern",
    file: "src/lower/mir/tidy.rs",
    find: "            StmtKind::If(test, ..) if !test.span.is_none() => stmt.span = test.span,\n",
    replace: "",
    tests: ["test/emission.test.ts", "-t", "source map points from fib.js"],
  },
  {
    name: "member-read-in-place-unfolded",
    breaks:
      "a field of an object read in place, a `dyn` field's pair, stays `{ value: f.inner, impl: f.impl }.impl`",
    file: "src/lower/mir/tidy.rs",
    find: "        && js::is_read_pair(props)\n",
    replace: "        && js::is_read_pair(props)\n        && false\n",
    tests: ["test/corpus.test.ts", "-t", "^unsized_struct_dyn.rs$"],
    snapshots: true,
  },
  {
    name: "member-of-any-object-read-in-place",
    breaks:
      "a field of any object read in place is folded, a temporary's box on the left of `=` too, `42 = ..`, which the printer can't write",
    file: "src/lower/mir/tidy.rs",
    find: "        && js::is_read_pair(props)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^temporary_borrows.rs$"],
  },
  {
    name: "written-first-of-a-switch",
    breaks:
      "an arm of a `match` of constants MIR tests first, written after one that leaves, is turned before `switches` reads the chain, which then reads it otherwise",
    file: "src/lower/mir/tidy.rs",
    find: "        if switched(test, first.span, switches) {\n            continue;\n        }\n",
    replace: "",
    // A turned kind's test is its case now too (rd70), so the typeof
    // switch's test holds it no more; a `match` of a `flatten`'s states
    // does.
    tests: ["test/corpus.test.ts", "-t", "^flatten_from_its_back.rs$"],
    snapshots: true,
  },
  {
    name: "again-stages-not-run",
    breaks:
      "the stages a stage asks to run again, `scopes`'s, don't, so what a guard's `try` held apart stays apart",
    file: "src/lower/mir/tidy.rs",
    find: "        if !stage.again.is_empty() && hints.again.take() {\n",
    replace: "        if false {\n",
    tests: [
      "test/lowering-drop-guards.test.ts",
      "-t",
      "a guard's fields are its variables",
    ],
  },
  {
    name: "again-stages-unchecked",
    breaks: "a stage run again stops nowhere `--tidy-until` names it",
    file: "src/lower/mir/tidy.rs",
    find: "                if run_stage(body, hints, again, &again_name(again)) {\n",
    replace: "                if run_stage(body, hints, again, again.name) {\n",
    tests: [
      "test/lowering-unwinds.test.ts",
      "-t",
      "a stage read again is its own",
    ],
  },
  {
    name: "tidy-until-ignored",
    breaks:
      "`--tidy-until` stops nowhere, so each stage's JS that's checked is the finished JS (ADR 0659, amended)",
    file: "src/lower/mir/tidy.rs",
    find: "        if hints.until == Some(&*pass_name(name, pass)) {\n",
    replace: "        if false {\n",
    tests: ["test/tidy-stages.test.ts", "-t", "isn't its JS after the last"],
  },
  {
    name: "arrow-of-a-value-kept-a-block",
    breaks:
      "`|| log(&seen, 1)` of a function of the crate's is `() => { log(seen, 1); }`, not its call's value as written (ADR 0883)",
    file: "src/lower/mir/tidy.rs",
    find: "    if matches!(e.kind, ExprKind::Arrow(..)) && of(e) == Some(crate::lower::jsx::OneCall::Value) {\n",
    replace: "    if matches!(e.kind, ExprKind::Arrow(..)) && false {\n",
    tests: ["test/lowering-arrows.test.ts"],
  },
  {
    name: "arrow-of-a-statement-its-value",
    breaks:
      "`|| { log(&seen, 2); }` is `() => log(seen, 2)`, a statement made a value (ADR 0883)",
    file: "src/lower/mir/tidy.rs",
    find: "    if matches!(e.kind, ExprKind::Arrow(..)) && of(e) == Some(crate::lower::jsx::OneCall::Value) {\n",
    replace:
      "    if matches!(e.kind, ExprKind::Arrow(..)) && of(e).is_some() {\n",
    tests: ["test/lowering-arrows.test.ts"],
  },
  {
    name: "placed-read-declared-apart",
    breaks:
      "a placed read's `try` gone, its variable stays `let previous; previous = current;`, not `const previous = current;` (#408)",
    file: "src/lower/mir/tidy.rs",
    find: "    if unthrown {\n        bindings::declarations(body);\n",
    replace: "    if false {\n        bindings::declarations(body);\n",
    tests: ["test/crates.test.ts", "-t", "settles its cleanup"],
  },
  {
    name: "tail-if-let-comments-dropped",
    breaks:
      "a comment after a function's last `if let`, React DOM's Float's `// We don't error because ..`, is dropped: its JS `if` is where the pattern is, not the `if` to its value's end (rd288)",
    file: "src/lower/mir/tidy.rs",
    find: "            matches!(s.kind, StmtKind::If(..)) && !s.span.is_none() && done.lo <= s.span.lo && s.span.hi <= done.hi\n",
    replace:
      "            matches!(s.kind, StmtKind::If(..)) && !s.span.is_none() && done.lo == s.span.lo && s.span.hi <= done.hi\n",
    tests: [
      "test/lowering-comments-ends.test.ts",
      "-t",
      "last if is at its end",
    ],
  },
  {
    name: "negations-evaluation-lost",
    breaks: "a unary remade, `+m` of `js::to_number(m)`, forgets that JS may throw of it, so unread it goes and `catch_unwind` of it is never `Err` (#462)",
    file: "src/lower/mir/tidy.rs",
    find: "        let facts = made.facts.and(e.facts.of_evaluation());\n",
    replace: "        let facts = made.facts;\n",
    tests: ["test/lowering-unwind-arms.test.ts", "-t", "a caught value nothing reads that may throw"],
  },
  {
    name: "tidy-until-cleanups-unguarded",
    breaks: "a body tidy stops in early, `--tidy-until`'s, runs its cleanups unguarded, so a drop's panic as a panic unwinds replaces it where Rust aborts (#847)",
    file: "src/lower/mir/tidy.rs",
    find: "    let stopped = |body: &mut Vec<Stmt>| (stages::GUARDED_CLEANUPS.run)(body, hints);\n",
    replace: "    let stopped = |_: &mut Vec<Stmt>| {};\n",
    tests: ["test/tidy-stages.test.ts", "-t", "unwinding_drop_aborts_locals"],
  },
  {
    name: "contracts-forget-too",
    breaks:
      "a contract is checked of what `--without-known` forgot: a `finally` the lowering wrote of what never throws is taken for unguarded without `NeverThrows`, and the compile panics where its JS is right (ADR 1127)",
    file: "src/lower/mir/tidy.rs",
    find: "            if let Some(why) = crate::js::remembering(|| (contract.broken)(body)) {\n",
    replace: "            if let Some(why) = (contract.broken)(body) {\n",
    tests: ["test/tidy-stages.test.ts", "-t", "flattened_state_got_mut after each"],
  },
  {
    name: "tidy-fired-unhooked",
    breaks: "tidy's stages record no pass, so `--tidy-fired` says none fired (#1461)",
    file: "src/lower/mir/tidy.rs",
    find: "        crate::lower::session::tidy_fired::after(was, body, || pass_name(name, pass));\n",
    replace: "",
    tests: ["test/tidy-stages.test.ts", "-t", "tidy-fired"],
  },
];
