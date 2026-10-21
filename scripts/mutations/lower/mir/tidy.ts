// Mutations of src/lower/mir/tidy.rs (ADR 0364).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "tidy-handle-assign-unseen",
    breaks: "a variable a handle's setter assigns is taken for one nothing sets after a tuple holds it, and read after the setter ran",
    file: "src/lower/mir/tidy.rs",
    find: "        if let ExprKind::Handle(place) | ExprKind::Pair(place, _) = &e.kind\n",
    replace: "        if false && let ExprKind::Handle(place) | ExprKind::Pair(place, _) = &e.kind\n",
    tests: ["test/corpus.test.ts", "-t", "tuple_scrutinee_calls"],
  },
  {
    name: "mir-tidy-branch-values-kept",
    breaks: "`if (c) { x = a; } else { x = b; }` stays two branches, not `x = c ? a : b`",
    file: "src/lower/mir/tidy.rs",
    find: "        bottom_up(body, &mut |stmts| branch_values(stmts, returns, booleans, made));\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","strings from MIR is the"],
  },
  {
    name: "mir-tidy-inlined-past-changed-reads",
    breaks: "a temporary that does something is written past reads of what it changes, `d.n + reset(d)`",
    file: "src/lower/mir/tidy.rs",
    find: "    let plain_root = matches!(&root.kind, ExprKind::Var(v) if plain.contains(v) && !set.contains(v));\n",
    replace: "    let plain_root = true;\n",
    tests: ["test/corpus.test.ts","-t","assignment_order"],
  },
  {
    name: "mir-tidy-falsy-temporary-kept",
    breaks: "a temporary given where what's falsy is skipped stays `test ? value : undefined`, not `test && value`",
    file: "src/lower/mir/tidy.rs",
    find: "                *value = Expr::bin(Op::And, test.tested(), (**then).clone());\n",
    replace: "",
    tests: ["test/bindings.test.ts","-t","skips what's falsy"],
  },
  {
    name: "mir-tidy-and-tests-off",
    breaks: "`if (a) { if (b) { s } }` of one condition stays nested, not `if (a && b)`",
    file: "src/lower/mir/tidy.rs",
    find: "    bottom_up(body, &mut |stmts| and_tests(stmts, conditions));\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","if-let chains and loops from MIR|a let chain from MIR is one test"],
  },
  {
    name: "mir-tidy-unused-let-kept",
    breaks: "a `let iter;` nothing assigns any more stays declared",
    file: "src/lower/mir/tidy.rs",
    find: "        stmts.retain(|stmt| !matches!(&stmt.kind, StmtKind::Let(name, None) if !mentions.contains_key(name)));\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","if-let chains and loops from MIR"],
  },
  {
    name: "mir-tidy-innermost-jumps-labeled",
    breaks: "a `continue l` of the innermost loop keeps its label, and so the loop",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut |stmts| continued(stmts, continues));\n    js::statement_lists(body, &mut |stmts| tail_jumps(stmts));\n    js::statement_lists(body, &mut |stmts| innermost_jumps(stmts));\n",
    replace: "    js::statement_lists(body, &mut |stmts| continued(stmts, continues));\n    js::statement_lists(body, &mut |stmts| tail_jumps(stmts));\n",
    tests: ["test/snapshots.test.ts", "-t", "calc"],
    snapshots: true,
  },
  {
    name: "mir-tidy-named-temporaries-off",
    breaks: "`const _4 = e; if (_4 != null) { const found = _4; … }` stays, not `const found = e;`",
    file: "src/lower/mir/tidy.rs",
    find: "    alias_variables(body);\n    named_temporaries(body);\n",
    replace: "    alias_variables(body);\n",
    tests: ["test/corpus.test.ts"],
    snapshots: true,
  },
  {
    name: "mir-tidy-guarded-off",
    breaks: "a `let … else` stays a labeled block holding the rest of the function",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut guarded);\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","let-else from MIR is a guard"],
  },
  {
    name: "mir-tidy-tested-kept",
    breaks: "a test a skips-falsy call reads alone keeps its `!!`, `(!!error || ready) && \"b\"`",
    file: "src/lower/mir/tidy.rs",
    find: "                *value = value.tested();\n",
    replace: "",
    tests: ["test/bindings.test.ts","-t","skips what's falsy"],
  },
  {
    name: "mir-tidy-filter-or-kept",
    breaks: "`(x ? x : undefined) ?? y` stays, not `x || y`",
    file: "src/lower/mir/tidy.rs",
    find: "            && truthy_or_undefined(kept)\n",
    replace: "            && false\n",
    tests: ["test/lowering.test.ts","-t","De Morgan"],
  },
  {
    name: "mir-tidy-lets-first",
    breaks: "each `let` stays declared first, not where it's first named",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut sunk_lets);\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","declared where it.s first named"],
  },
  {
    name: "mir-tidy-late-consts-kept",
    breaks: "a `const` declarations makes last stays, `const _27 = e; _24 = _27;`",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut |stmts| guarded_lets(stmts, captured));\n    js::statement_lists(body, &mut |stmts| {\n        inlined(stmts, &changed, &set, plain, captured, &paired)\n    });\n",
    replace: "    js::statement_lists(body, &mut |stmts| guarded_lets(stmts, captured));\n",
    tests: ["test/lowering.test.ts","-t","else if. given as a value"],
  },
  {
    name: "mir-tidy-plain-cells-boxed",
    breaks: "a cell only its function reads and sets stays `{ value }`",
    file: "src/lower/mir/tidy.rs",
    find: "    plain_cells(body);\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","only its function reads and sets"],
  },
  {
    name: "mir-tidy-callback-unnamed",
    breaks: "a hook's closure a `let` names stays `const tick = () => …; useEffect(tick, …)`",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut |stmts| named_callbacks(stmts, callbacks));\n",
    replace: "",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-tidy-constant-objects-block",
    breaks: "a call isn't read in place past an object of constants before it, `const _3 = unquoted(..)`",
    file: "src/lower/mir/tidy.rs",
    find: "read_past_calls(result, set, &plain)\n        }\n        _ => e.is_made_of_constants(),\n",
    replace: "read_past_calls(result, set, &plain)\n        }\n        _ => e.is_constant(),\n",
    tests: ["test/lowering.test.ts","-t","past an object of constants"],
  },
  {
    name: "mir-tidy-late-tail-returns",
    breaks: "a `return;` the late passes leave at a function's end stays",
    file: "src/lower/mir/tidy.rs",
    find: "    // What those leave at the end, a `return;` the program didn't write.\n    tail_returns(body, returns);\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","of .matches!. from MIR"],
  },
  {
    "name": "late-while-tests-off",
    "breaks": "a `while` whose test MIR held first, `const _7 = m.length;`, stays `while (true)` and a `return`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    // > n)) { .. } }`'s of `const _7 = m.length;`.\n    js::statement_lists(body, &mut |stmts| while_tests(stmts, written));\n",
    "replace": "    // > n)) { .. } }`'s of `const _7 = m.length;`.\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a while in a closure from MIR keeps its test"
    ]
  },
  {
    "name": "builtin-callee-blocks-inline",
    "breaks": "a call's temporary given to `Object.fromEntries` stays a `const`, and no object literal is made of it",
    "file": "src/lower/mir/tidy.rs",
    "find": "\"Object\" | \"Math\"",
    "replace": "\"Object!\" | \"Math\"",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "read in place by a builtin's function"
    ]
  },
  {
    "name": "shown-child-conditional",
    "breaks": "a child shown where a test holds is `shown ? hint : undefined`, not `shown && hint`",
    "file": "src/lower/mir/tidy.rs",
    "find": "                    && matches!(none.kind, ExprKind::Undefined)\n                {\n                    let test = match is_boolean(test, booleans) {",
    "replace": "                    && false\n                {\n                    let test = match is_boolean(test, booleans) {",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "late-not-unsimplified",
    "breaks": "`!matches!(t, \"pre\")` read in place stays `!(t === \"pre\")`",
    "file": "src/lower/mir/tidy.rs",
    "find": "                ..Expr::unary(*op, (**arg).clone())\n",
    "replace": "                kind: ExprKind::Unary(*op, arg.clone()),\n                reads: false,\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "negated matches, string methods"
    ]
  },
  {
    "name": "read-handles-off",
    "breaks": "a cell field a pattern binds, only read, is a handle, `{ get value() { .. } }`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    read_handles(body);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a cell field a let pattern binds"
    ]
  },
  {
    "name": "plain-field-blocks-inline",
    "breaks": "a prop MIR holds for a borrow stays a `const _14`, read past `meta.tag` of a plain value",
    "file": "src/lower/mir/tidy.rs",
    "find": "    let plain_root = matches!(&root.kind, ExprKind::Var(v) if plain.contains(v) && !set.contains(v));\n",
    "replace": "    let plain_root = false;\n",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-tidy-import-blocks-inline",
    "breaks": "a prop MIR holds for a borrow stays a `const _13`, read past an imported tag, `<Seo>`",
    "file": "src/lower/mir/tidy.rs",
    "find": "        ExprKind::Symbol(_) => true,\n        ExprKind::Arrow(..)",
    "replace": "        ExprKind::Arrow(..)",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-tidy-spread-spreads-kept",
    "breaks": "a copy MIR made of a dictionary spread stays `{ ...base, ...{ ...extra } }`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::each_expr_mut(body, &mut spread_spreads);\n",
    "replace": "",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-tidy-checked-sets-kept",
    "breaks": "an element's place MIR checks then sets stays two statements, `$index(args, 0); args[0] = v;`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut checked_sets);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "checked element set is one statement"
    ]
  },
  {
    "name": "mir-tidy-written-first-unswapped",
    "breaks": "an if-let chain from MIR keeps its first arm last, the panic's test first: `if (kind.TAG !== \"Text\") { .. throw } message = ..`",
    "file": "src/lower/mir/tidy.rs",
    "find": "        js::statement_lists(body, &mut |stmts| written_first(stmts));\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "if-let chain ending in a panic"
    ]
  },
  {
    "name": "mir-tidy-tail-elses-kept",
    "breaks": "an else-if chain's arm from MIR breaks its block, `if (c) { s; break bb19; } throw ..`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut |stmts| tail_elses(stmts));\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "if-let chain ending in a panic"
    ]
  },
  {
    "name": "mir-tidy-typeof-null-kept",
    "breaks": "a typeof test from MIR keeps the null check before it, `given != null && typeof given === \"string\"`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::each_expr_mut(body, &mut typeof_tests);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "typeof test after a null check"
    ]
  },
  {
    "name": "mir-tidy-chained-checks-kept",
    "breaks": "an else-if chain's arm before a panic from MIR is a check in its else, `if (className !== \"language-css\") throw ..`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut |stmts| chained_checks(stmts));\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "arm before a panic stays an else if"
    ]
  },
  {
    "name": "mir-tidy-sibling-element-blocks",
    "breaks": "a prop MIR holds for a borrow stays a `const _16`, read past a sibling element, `<Search isOpen={show} />`",
    "file": "src/lower/mir/tidy.rs",
    "find": "        ExprKind::Jsx(_) => inert_element(e, set),\n",
    "replace": "",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-tidy-written-chains-kept",
    "breaks": "a match's arms as statements from MIR keep their first arm last, `if (message.TAG !== \"resize\") { .. } else { .. }`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut |stmts| written_chains(stmts));\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "arms as statements chain in written order|an else-if at a loop's end from MIR stays one past a written continue"
    ]
  },
  {
    "name": "mir-tidy-guarded-lets-kept",
    "breaks": "an element MIR makes only where a test holds stays `let className$2; if (error != null) { .. }`, the props before it held too",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut |stmts| guarded_lets(stmts, captured));\n",
    "replace": "",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-tidy-implied-tests-kept",
    "breaks": "a value read where its test holds keeps the test, `error != null && <div className={error != null ? cn(..) : undefined}>`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::each_expr_mut(body, &mut |e| implied_tests(e, &changed));\n",
    "replace": "",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-tidy-copied-bindings-kept",
    "breaks": "an if-let's binding of a variable MIR copies stays `const error = raw;`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut |stmts| copied_bindings(stmts));\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "copy binding of a variable is the variable"
    ]
  },
  {
    "name": "mir-tidy-tuple-reads-kept",
    "breaks": "children MIR holds as a tuple stay `const children = [..]`, read `children[0]`, and its `then` closure a `const value`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut |stmts| {\n        tuple_reads(stmts, &changing, &set_elsewhere, captured)\n    });\n",
    "replace": "",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-tidy-dyn-pairs-kept",
    "breaks": "dyn values MIR gives JS stay `{ value, impl }` pairs, unwrapped with `.map((item) => item.value)`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut |stmts| unwrapped_values(stmts));\n",
    "replace": "",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-tidy-closure-made-blocks",
    "breaks": "a closure's making, `const value = () => ..`, blocks props held before it, `const value$3 = cn(..)`",
    "file": "src/lower/mir/tidy.rs",
    "find": "        ExprKind::Arrow(..) | ExprKind::AsyncArrow(..) => true,\n        ExprKind::Member(of, _) => {\n",
    "replace": "        ExprKind::Member(of, _) => {\n",
    "tests": ["test/jsx.test.ts"]
  },
  {
    "name": "mir-tidy-choice-blocks",
    "breaks": "a conditional of parameters, `selected ? \"page\" : undefined`, blocks a prop held after it",
    "file": "src/lower/mir/tidy.rs",
    "find": "        ExprKind::Cond(a, b, c) => past(a) && past(b) && past(c),\n",
    "replace": "",
    "tests": ["test/jsx.test.ts"]
  },
  {
    "name": "mir-tidy-lets-block",
    "breaks": "a `let` no closure sets, `target`, blocks a prop held after it, as any variable set again",
    "file": "src/lower/mir/tidy.rs",
    "find": "    let set = closures_set(body);\n",
    "replace": "    let set = changed.clone();\n",
    "tests": ["test/jsx.test.ts"]
  },
  {
    "name": "mir-tidy-returned-consts-kept",
    "breaks": "what a function gives from MIR stays `const result = e; return result;`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut |stmts| returned_consts(stmts, result));\n",
    "replace": "",
    "tests": ["test/lowering.test.ts", "-t", "or chain of a getter's bool"]
  },
  {
    "name": "mir-tidy-concise-handlers-kept",
    "breaks": "a held handler of one call from MIR keeps its block, `() => { f(); }`",
    "file": "src/lower/mir/tidy.rs",
    "find": "                    && concise.contains(&value.span)\n",
    "replace": "                    && false\n",
    "tests": ["test/jsx.test.ts"]
  },
  {
    "name": "inert-element-chosen-not",
    "breaks": "a child chosen of elements, `open ? <b /> : <i />`, stops classes held for a later child being read in place",
    "file": "src/lower/mir/tidy.rs",
    "find": "        ExprKind::Cond(a, b, c) => [a, b, c].iter().all(|e| inert_element(e, changing)),\n",
    "replace": "",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "spread-items-not-read-past-calls",
    "breaks": "`...first` of an array whose items nothing changes stops a later call's value being read in place",
    "file": "src/lower/mir/tidy.rs",
    "find": " || plain.contains(&format!(\"...{v}\"))",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "arrays joined are one array of their items"
    ]
  },
  {
    "name": "jsx-child-truth-test-kept",
    "breaks": "a child shown where text is truthy stays `href ? <a /> : undefined`, not `!!href && <a />`",
    "file": "src/lower/mir/tidy.rs",
    "find": "                        false => Expr::unary(UnaryOp::Not, Expr::unary(UnaryOp::Not, (**test).clone())),",
    "replace": "                        false => continue,",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "table-read-after-inlining",
    "breaks": "a match's table read MIR has already read in place, a child's, stays its chain of tests, not `TITLES[kind]`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    // what it's given to reads it in place.\n    js::statement_lists(body, &mut |stmts| table_reads(stmts, tables));\n",
    "replace": "    // what it's given to reads it in place.\n",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "plain-reads-held",
    "breaks": "a field of plain data MIR holds, `const t = q.title`, stays held, destructured, not read in place",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut |stmts| plain_reads(stmts, plain, captured));\n",
    "replace": "",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "rest-parts-apart",
    "breaks": "a `let`'s pattern's rest and what it binds stay two patterns, `const { href: _href, ...rest } = p; const { href } = p;`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut rest_parts);\n",
    "replace": "",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-labels-by-block",
    "breaks": "a loop the program labeled `'rows` from MIR is labeled by its block, `bb2`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut |stmts| named_labels(stmts, written));\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "nested loops from MIR read as written"
    ]
  },
  {
    "name": "mir-closure-values-first",
    "breaks": "a closure's `$iter(g)` from MIR is `g.values()` before its function sees `flatMap` take it",
    "file": "src/lower/mir/tidy.rs",
    "find": "    if !closure {\n",
    "replace": "    if true {\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "iterator chains from MIR are array methods"
    ]
  },
  {
    "name": "mir-loop-exit-returned-inside",
    "breaks": "a loop left by `break` before the function's value from MIR returns the value inside the loop",
    "file": "src/lower/mir/tidy.rs",
    "find": "    js::statement_lists(body, &mut exits_after);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a loop's break from MIR is a break"
    ]
  },
  {
    "name": "mir-or-tests-nested",
    "breaks": "`\"a\" | \"e\" => \u2026` from MIR is a test failing for each, one inside another, the arm after them",
    "file": "src/lower/mir/tidy.rs",
    "find": "    bottom_up(body, &mut or_tests);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a string's or-pattern from MIR"
    ]
  },
  {
    "name": "mir-tidy-handle-in-place-kept",
    "breaks": "`Cell::set` of `&self.n` stays a getter and setter object made to be written once, not `t.n = v`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    handles_in_place(body);\n",
    "replace": "",
    tests: ["test/lowering.test.ts", "-t", "a handle used in place from MIR is its place"],
  },
  {
    name: "mir-tidy-unread-map-kept",
    breaks: "a map `next_back` stepped the source of instead is a `const` nothing reads",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut forwarded_values);\n    unread_maps(body);\n",
    replace: "    js::statement_lists(body, &mut forwarded_values);\n",
    tests: ["test/lowering.test.ts", "-t", "next_back of a map"],
  },
  {
    name: "mir-tidy-return-span-none",
    breaks: "MIR's `return` of a value put in it maps to no Rust",
    file: "src/lower/mir/tidy.rs",
    find: "                StmtKind::Return(Some(value)) if stmt.span.is_none() => stmt.span = value.span,\n",
    replace: "",
    tests: ["test/emission.test.ts","-t","source map points from fib.js"],
  },
  {
    name: "mir-tidy-if-span-of-test-kept",
    breaks: "MIR's `if` maps to its condition or its `match`, not `if c` or its arm's pattern",
    file: "src/lower/mir/tidy.rs",
    find: "                StmtKind::If(test, ..) if !test.span.is_none() => stmt.span = test.span,\n",
    replace: "",
    tests: ["test/emission.test.ts","-t","source map points from fib.js"],
  },
  {
    name: "mir-tidy-nested-block-returns-kept",
    breaks: "MIR's returns in a loop go through one exit, a labeled block",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut |stmts| returned_in_branches(stmts, result));\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","returns in a loop are where"],
  },
  {
    name: "mir-tidy-while-heads-kept",
    breaks: "MIR's loop tested at its head stays `while (true)`",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut while_heads);\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","returns in a loop are where"],
  },
  {
    name: "mir-tidy-headed-labels-kept",
    breaks: "MIR's jumps of a loop tested at its head keep its label: calc's `continue bb4`",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut |stmts| innermost_jumps(stmts));\n    settle_jumps(body);\n    // `!(b >= 48)`",
    replace: "    settle_jumps(body);\n    // `!(b >= 48)`",
    tests: ["test/snapshots.test.ts", "-t", "calc"],
    snapshots: true,
  },
  {
    name: "mir-tidy-late-labels-kept",
    breaks: "a loop's label that an inner loop's way out, its `break` now, no longer names stays: `bb3: for (..)`",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut |stmts| innermost_jumps(stmts));\n    settle_jumps(body);\n    // `!(b >= 48)`",
    replace: "    js::statement_lists(body, &mut |stmts| innermost_jumps(stmts));\n    // `!(b >= 48)`",
    tests: ["test/corpus.test.ts", "-t", "^(nested_loop_jumps|writer_callables).rs$"],
    snapshots: true,
  },
  {
    name: "mir-tidy-steps-out-of-order",
    breaks: "MIR's loop body tests what's written later first",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut |stmts| written_order_steps(stmts));\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","returns in a loop are where"],
  },
  {
    name: "mir-tidy-tried-last",
    breaks: "MIR's `?` in a branch is `if (ok) { s } else { return r; }`, not a guard",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut tried_first);\n",
    replace: "",
    tests: ["test/snapshots.test.ts"],
  },
  {
    name: "mir-tidy-unread-made-kept",
    breaks: "MIR's array made and never read is kept, `const value = [loud(1)]`",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut |stmts| unread_made(stmts, &mentions));\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","a value made only to be dropped is what making it does"],
  },
  {
    name: "mir-tidy-decisions-off",
    breaks: "a match MIR tests as a tree stays a labeled block each leaf breaks out of",
    file: "src/lower/mir/tidy.rs",
    find: "    bottom_up(body, &mut decisions::decisions);\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "arms of more than one pattern"],
  },
  {
    name: "mir-tidy-default-values-off",
    breaks: "`matches!(v, Some(x) if ..)`'s `false` stays the block's last statement, the block labeled",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut |stmts| decisions::default_values(stmts));\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "no arm sets it"],
  },
  {
    name: "loop-exits-taken-once",
    breaks: "a loop's end of the shape it takes later, `if (c) { a; continue; } return x;`, stays so, not `if (!c) { break; } a` and the `return` after it",
    file: "src/lower/mir/tidy.rs",
    find: "    // A loop's early exits again, of what's taken its shape since.\n    js::statement_lists(body, &mut early_returns);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(calculator|kept_generic_iterators|interpreter).rs$"],
    snapshots: true,
  },
  {
    name: "tuple-part-binding-unnamed",
    breaks: "a tuple's part held, which an arm binds, `const x = value;`, keeps the temporary's name",
    file: "src/lower/mir/tidy.rs",
    find: "    // takes, by that name.\n    named_temporaries(body);\n",
    replace: "    // takes, by that name.\n",
    tests: ["test/corpus.test.ts", "-t", "tuple_scrutinee_calls"],
    snapshots: true,
  },
  {
    name: "turned-never",
    breaks: "`!(b >= 48) || !(b <= 57)` of a byte stays so, not `b < 48 || b > 57`",
    file: "src/lower/mir/tidy.rs",
    find: "    js::each_expr_mut(body, &mut |e| turned(e, numbers));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "byte_ascii|loop_exits"],
    snapshots: true,
  },
  {
    name: "mir-tidy-eager-chains-off",
    breaks: "`$iter(xs).map(f).toArray()` stays, not `xs.map(f)`",
    file: "src/lower/mir/tidy.rs",
    find: "    js::each_expr_mut(body, &mut crate::lower::std_types::range::range_items);\n    eager_chains(body);\n    js::statement_lists(body, &mut |stmts| eager_spreads(stmts));\n",
    replace: "    js::each_expr_mut(body, &mut crate::lower::std_types::range::range_items);\n    js::statement_lists(body, &mut |stmts| eager_spreads(stmts));\n",
    tests: ["test/lowering.test.ts", "-t", "iterator chains from MIR|a lazy chain from MIR spread once"],
  },
  {
    name: "mir-tidy-guards-kept",
    breaks: "a guard of a handle made for it alone stays `$borrowMut({ get value() { \u2026 } }, true)`",
    file: "src/lower/mir/tidy.rs",
    find: "    // A guard of a handle made for it alone: the place it's a handle on.\n    crate::lower::std_types::cell::unguarded(body);\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "field only ever borrowed for a moment"],
  },
  {
    name: "late-eager-chains-off",
    breaks: "an array's iterator read in place late stays `$lent($iter(xs)).some(f)`",
    file: "src/lower/mir/tidy.rs",
    find: "    iterators::consumed_into_iters(body);\n    js::each_expr_mut(body, &mut crate::lower::std_types::range::range_items);\n    eager_chains(body);\n    // Of each, a callback",
    replace: "    iterators::consumed_into_iters(body);\n    js::each_expr_mut(body, &mut crate::lower::std_types::range::range_items);\n    // Of each, a callback",
    tests: ["test/lowering.test.ts", "-t", "an array's iterator read in place late"],
  },
  {
    name: "mir-map-count-handle",
    breaks: "`*m.entry(k).or_insert(0) += 1` from MIR is a handle on the key's value, `$mutGet(m, k).value = \u2026`",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut crate::lower::std_types::map::counted_entries);\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "a count kept in a map from MIR"],
  },
  {
    name: "mir-tidy-applied-values-kept",
    breaks: "a value `{:?}` reads twice, given as it's made, stays a function called in place, `((result) => ..)(f())`, not `const value = f();`",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut |stmts| applied_values(stmts, &set, plain, &mut taken));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^option_result_methods.rs$"],
    snapshots: true,
  },
  {
    name: "mir-tidy-template-unread-past",
    breaks: "text made of what calls can't change, `${a} ${b}`, isn't read past a call, and what follows it stays a function called in place",
    file: "src/lower/mir/tidy.rs",
    find: "        ExprKind::Template(_, values, _) => values.iter().all(past),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^format_args_order.rs$"],
    snapshots: true,
  },
  {
    name: "mir-tidy-console-unread-past",
    breaks: "`console.log` isn't read past a call, and a value it prints `{:?}` of stays a function called in place",
    file: "src/lower/mir/tidy.rs",
    find: "\"Number\" | \"String\" | \"console\"))",
    replace: "\"Number\" | \"String\"))",
    tests: ["test/corpus.test.ts", "-t", "^option_result_methods.rs$"],
    snapshots: true,
  },
  {
    name: "mir-tidy-pure-call-unread-past",
    breaks: "a call of the runtime's that only reads, `$debugStr(s)`, isn't read past a call",
    file: "src/lower/mir/tidy.rs",
    find: "        ExprKind::Call(callee, args) if callee.helper_name().is_some_and(crate::runtime::is_pure) => {\n            args.iter().all(past)\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^text_edits.rs$"],
    snapshots: true,
  },
  {
    name: "mir-tidy-item-unread-past",
    breaks: "an item of a plain value, `pair[0]`, isn't read past a call, as its field is, and what follows stays a function called in place",
    file: "src/lower/mir/tidy.rs",
    find: "        ExprKind::Index(_, index) => plain_root && past(index),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^const_generic_traits.rs$"],
    snapshots: true,
  },
  {
    name: "tidy-applied-call-not-past",
    breaks: "a function called at once on a held value is taken to read what a call may change, and the `Option` formatted after it stays a function called in place",
    file: "src/lower/mir/tidy.rs",
    find: "            past(given) && read_past_calls(result, set, &plain)\n",
    replace: "            false\n",
    tests: ["test/corpus.test.ts", "-t", "^iter_next_back.rs$"],
    snapshots: true,
  },
  {
    name: "cell-first-value-apart",
    breaks: "a cell a pointer's taken to is `const x = {}; x.value = 1;`, not `const x = { value: 1 };`",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut first_cell_values);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^raw_pointers.rs$"],
    snapshots: true,
  },
  {
    name: "tail-continues-unwritten-kept",
    breaks: "a `continue` the program didn't write, left at a loop body's end by a block returned in place, stays",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut |stmts| tail_continues(stmts, continues));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^interpreter.rs$"],
    snapshots: true,
  },
  {
    name: "member-read-in-place-unfolded",
    breaks: "a field of an object read in place, a `dyn` field's pair, stays `{ value: f.inner, impl: f.impl }.impl`",
    file: "src/lower/mir/tidy.rs",
    find: "            && js::is_read_pair(props)\n",
    replace: "            && js::is_read_pair(props)\n            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^unsized_struct_dyn.rs$"],
    snapshots: true,
  },
  {
    name: "scope-lets-not-declared-where-made",
    breaks: "a `let x;` a scope's `try` reads stays apart from where it's made, `let tmp; tmp = ids(v);`",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut unwinds::scoped_drops);\n    bindings::declarations(body);\n",
    replace: "    js::statement_lists(body, &mut unwinds::scoped_drops);\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_destructors.rs$"],
    snapshots: true,
  },
  {
    "name": "mir-temporaries-unnamed",
    "breaks": "a temporary MIR holds is named `_4`, not `value`",
    "file": "src/lower/mir/tidy.rs",
    "find": "    bindings::readable_temporaries(params, body);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "held temporary is named"
    ]
  },
  {
    name: "member-of-any-object-read-in-place",
    breaks: "a field of any object read in place is folded, a temporary's box on the left of `=` too, `42 = ..`, which the printer can't write",
    file: "src/lower/mir/tidy.rs",
    find: "            && js::is_read_pair(props)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^temporary_borrows.rs$"],
  },
  {
    name: "late-unreachable-kept",
    breaks: "what a jump settled late leaves past it stays, `break bb67; return f;`",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut unreachable_after);\n    js::statement_lists(body, &mut |stmts| jumps_past_blocks(stmts));\n",
    replace: "    js::statement_lists(body, &mut |stmts| jumps_past_blocks(stmts));\n",
    tests: ["test/corpus.test.ts", "-t", "^(nested_loop_jumps|writer_callables|interpreter|rc_counted_uses|loop_drops).rs$"],
    snapshots: true,
  },
  {
    name: "outer-break-not-early",
    breaks: "an inner loop whose way out is its `break` now ends `if (value) { .. break; } break;`, not `if (!value) { break; }` first",
    file: "src/lower/mir/tidy.rs",
    find: "    js::statement_lists(body, &mut |stmts| outer_continues(stmts, written));\n    js::statement_lists(body, &mut |stmts| early_breaks(stmts, false));\n",
    replace: "    js::statement_lists(body, &mut |stmts| outer_continues(stmts, written));\n",
    tests: ["test/corpus.test.ts", "-t", "^(nested_loop_jumps|writer_callables).rs$"],
    snapshots: true,
  },
  {
    name: "holed-lists-kept",
    breaks: "a list MIR took its other items out of is kept, `[undefined, ids(v)]`, read as `tmp[1]`",
    file: "src/lower/mir/tidy.rs",
    find: "    holed_lists(body);\n",
    replace: "",
    tests: ["test/lowering.test.ts", "-t", "a list of one item left is the item"],
  },
  {
    name: "unread-flag-kept-after-decided-tests",
    breaks: "a drop flag whose only test went with a decided drop stays, written and never read, `let bus$0$live = true`",
    file: "src/lower/mir/tidy.rs",
    find: "    flags::settled_flags(body, flags);\n    js::statement_lists(body, &mut shared_tails);\n",
    replace: "    js::statement_lists(body, &mut shared_tails);\n",
    tests: ["test/corpus.test.ts", "-t", "^subscriptions.rs$"],
    snapshots: true,
  },
];
