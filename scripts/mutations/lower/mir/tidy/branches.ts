// Mutations of src/lower/mir/tidy/branches.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-tidy-empty-then-kept",
    breaks: "`if (c) {} else { s }` stays, not `if (!c) { s }`",
    file: "src/lower/mir/tidy/branches.rs",
    find: "            StmtKind::If(test, then, Some(otherwise)) if then.is_empty() => {\n                StmtKind::If(test.negated_test(), otherwise, None)\n            }\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","branches from MIR read"],
  },
  {
    name: "mir-tidy-returned-chain-kept",
    breaks: "`if (a) { if (b) { return t; } } return f;` stays, not `return a && b ? t : f`",
    file: "src/lower/mir/tidy/branches.rs",
    find: "        stmts.push(StmtKind::Return(Some(value)).at(span));\n",
    replace: "        let _ = value;\n        return;\n",
    tests: ["test/lowering.test.ts","-t","branches from MIR read"],
  },
  {
    name: "mir-tidy-or-chain-kept",
    breaks: "`l: { if (!a) { if (!b) { s; break l; } } t }` stays, not `if (a || b) { t } else { s }`",
    file: "src/lower/mir/tidy/branches.rs",
    find: "            StmtKind::Labeled(label, body) if let Some((tests, otherwise, then)) = or_chain(&body, &label) => {\n",
    replace: "            StmtKind::Labeled(label, body) if let Some((tests, otherwise, then)) = or_chain(&body, &label).filter(|_| false) => {\n",
    tests: ["test/lowering.test.ts","-t","react.dev's Console clears its logs"],
  },
  {
    name: "mir-tidy-cond-false-kept",
    breaks: "`test ? b : false` of a boolean test stays, not `test && b`",
    file: "src/lower/mir/tidy/branches.rs",
    find: "        (_, ExprKind::Bool(false)) => Expr::bin(Op::And, truth(test), yes),\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","branches from MIR read"],
  },
  {
    name: "mir-tidy-and-tests-any-condition",
    breaks: "`if`s the program nests are merged into one `&&`",
    file: "src/lower/mir/tidy/branches.rs",
    find: "        let one = (conditions.iter()).any(|(first, then)| within(first, span) && within(then, inner.span));\n",
    replace: "        let one = true;\n",
    tests: ["test/lowering.test.ts","-t","if-let chains and loops from MIR"],
  },
  {
    name: "mir-tidy-temporary-ternary-kept",
    breaks: "an `if` the program wrote as a value, of an element, stays a `let` assigned in branches, not a conditional child",
    file: "src/lower/mir/tidy/branches.rs",
    find: "                    && ((small(yes) && small(no)) || is_temporary(x))\n",
    replace: "                    && small(yes) && small(no)\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-tidy-written-return-ternary",
    breaks: "`if (!g) { return; } return big;`, as the program wrote it, is `return g ? big : undefined`",
    file: "src/lower/mir/tidy/branches.rs",
    find: "        && !written(returns, last.span)\n        && !writes_return(std::slice::from_ref(last_if), returns)\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","value of two branches is returned"],
  },
  {
    name: "mir-tidy-let-else-return-nested",
    breaks: "a `let … else { return f }` stays `if (a) { if (b) { return t; } } return f;`, not a guard",
    file: "src/lower/mir/tidy/branches.rs",
    find: "        && let Some((tests, body)) = returning_chain(std::slice::from_ref(last_if))\n",
    replace: "        && let Some((tests, body)) = returning_chain(std::slice::from_ref(last_if)).filter(|_| false)\n",
    tests: ["test/lowering.test.ts","-t","let-else from MIR is a guard"],
  },
  {
    name: "mir-tidy-boolean-variable-unknown",
    breaks: "a variable of `booleans` isn't a `bool` to `conditional`, `!!expanded`",
    file: "src/lower/mir/tidy/branches.rs",
    find: "        ExprKind::Var(name) => booleans.contains(name),\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","value of two branches is returned"],
  },
  {
    name: "mir-tidy-truth-uncoerced",
    breaks: "`o ? true : r` of an object `o` is `o || r`, an object where a `bool` belongs",
    file: "src/lower/mir/tidy/branches.rs",
    find: "        false => Expr::unary(UnaryOp::Not, Expr::unary(UnaryOp::Not, test)),\n",
    replace: "        false => test,\n",
    tests: ["test/lowering.test.ts","-t","value of two branches is returned"],
  },
  {
    name: "mir-tidy-implied-guard-kept",
    breaks: "`if (x != null) { if (x === \"a\") { s } }` keeps the guard its chain says",
    file: "src/lower/mir/tidy/branches.rs",
    find: "            StmtKind::If(test, then, None) if !then.is_empty() && then.iter().all(|s| guarded_chain(s, test)) => {\n",
    replace: "            StmtKind::If(test, then, None) if false && !then.is_empty() && then.iter().all(|s| guarded_chain(s, test)) => {\n",
    tests: ["test/lowering.test.ts","-t","strings from MIR is the"],
  },
  {
    name: "mir-tidy-implied-by-or-unread",
    breaks: "`a || b` doesn't say what both its sides say, `x != null` of `x === \"d\" || x === \"i\"`",
    file: "src/lower/mir/tidy/branches.rs",
    find: "        (_, ExprKind::Binary(Op::Or, a, b)) => implies(a, test) && implies(b, test),\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","strings from MIR is the"],
  },
  {
    name: "mir-tidy-guarded-chain-else",
    breaks: "a chain with a last `else` drops its guard, its `else` then run where the guard fails",
    file: "src/lower/mir/tidy/branches.rs",
    find: "                && (otherwise.as_deref()).is_none_or(|rest| matches!(rest, [next] if guarded_chain(next, guard)))\n        }\n        _ => false,\n",
    replace: "                && (otherwise.as_deref()).is_none_or(|rest| matches!(rest, [next] if guarded_chain(next, guard)))\n        }\n        _ => true,\n",
    tests: ["test/lowering.test.ts","-t","strings from MIR is the"],
  },
  {
    name: "mir-tidy-filtered-guard-kept",
    breaks: "a let-else of what `filter` keeps tests `const h = x ? x : undefined` for `null`, not `!x`",
    file: "src/lower/mir/tidy/branches.rs",
    find: "            Op::LooseEq => otherwise.is_none() && then.last().is_some_and(ends),\n",
    replace: "            Op::LooseEq => false,\n",
    tests: ["test/lowering.test.ts","-t","what .filter. keeps"],
  },
  {
    name: "mir-tidy-filter-or-chain",
    breaks: "`(x ? x : undefined) ?? (y ? y : undefined) ?? z` isn't known as one, `x || y || z`",
    file: "src/lower/mir/tidy/branches.rs",
    find: "        ExprKind::Binary(Op::Coalesce, a, b) => truthy_or_undefined(a) && truthy_or_undefined(b),\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","De Morgan"],
  },
  {
    name: "mir-tidy-filtered-some-kept",
    breaks: "`if (h != null) { s }` of what `filter` keeps stays, `h` and its binding `const href = h`",
    file: "src/lower/mir/tidy/branches.rs",
    find: "                otherwise.as_deref().is_none_or(|o| js::mentions_in(o, h) == 0)\n                    && js::mentions_in(&stmts[i + 1..], h) == 0\n",
    replace: "                false\n",
    tests: ["test/jsx.test.ts"],
  },
  {
    name: "mir-tidy-else-after-return-kept",
    breaks: "`if (c) { return a; } else { s }`, its `else` kept, the let-else not a guard",
    file: "src/lower/mir/tidy/branches.rs",
    find: "        StmtKind::If(test, then, Some(otherwise)) if then.last().is_some_and(ends) => {\n",
    replace: "        StmtKind::If(test, then, Some(otherwise)) if false && then.last().is_some_and(ends) => {\n",
    tests: ["test/lowering.test.ts","-t","branches assign is returned from each"],
  },
  {
    name: "mir-tidy-let-else-of-written-returns",
    breaks: "`if c { return a; } else { return b; }`, both written, is `if (!c) { return b; } return a;`",
    file: "src/lower/mir/tidy/branches.rs",
    find: "        && !body.last().is_some_and(|s| matches!(s.kind, StmtKind::Return(_)) && written(returns, s.span))\n",
    replace: "",
    tests: ["test/lowering.test.ts","-t","string's parts from MIR"],
  },
  {
    "name": "table-reads-off",
    "breaks": "a `match` reading a table is MIR's tests of each variant, not `MAP[kind]`",
    "file": "src/lower/mir/tidy/branches.rs",
    "find": "    let within = |s: js::Span| table_of(s).is_some();",
    "replace": "    let within = |s: js::Span| false && table_of(s).is_some();",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a match reading a table from MIR"
    ]
  },
  {
    "name": "table-reads-returns-kept",
    "breaks": "a `match` of returns, each its variant's own name, stays a return for each, not `return kind;`",
    "file": "src/lower/mir/tidy/branches.rs",
    "find": "                stmts.splice(i..=i + returned, [StmtKind::Return(Some(read)).at(span)]);\n",
    "replace": "                let _ = (read, span);\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a match reading a table from MIR"
    ]
  },
  {
    "name": "table-read-own-names-off",
    "breaks": "a `match` giving each variant's own name isn't what's matched",
    "file": "src/lower/mir/tidy/branches.rs",
    "find": "        None if names == arms.len() => Some((**key).clone()),\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a match reading a table from MIR"
    ]
  },
  {
    "name": "table-read-mixed-arms",
    "breaks": "a `match` of a table's fields and a literal reads the table for the literal's variant too",
    "file": "src/lower/mir/tidy/branches.rs",
    "find": "        Some(table) if fields.len() == arms.len() =>",
    "replace": "        Some(table) =>",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a match reading a table from MIR"
    ]
  },
  {
    "name": "negated-and-untruthed",
    "breaks": "`!o ? false : f()` is `o && f()`, `undefined` where `o` is, not `false`",
    "file": "src/lower/mir/tidy/branches.rs",
    "find": "        (ExprKind::Bool(false), _) => Expr::bin(Op::And, truth(negate(test)), no),\n",
    "replace": "        (ExprKind::Bool(false), _) => Expr::bin(Op::And, negate(test), no),\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a truthy option's is_some given as a value"
    ]
  },
  {
    "name": "negated-value-untruthed",
    "breaks": "`!o ? false : true` is `o`, the object, not `true`",
    "file": "src/lower/mir/tidy/branches.rs",
    "find": "        (ExprKind::Bool(false), ExprKind::Bool(true)) => truth(negate(test)),\n",
    "replace": "        (ExprKind::Bool(false), ExprKind::Bool(true)) => negate(test),\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a truthy option's is_some given as a value"
    ]
  },
  {
    "name": "negated-or-untruthed",
    "breaks": "`o ? f() : true` of `is_none()`'s is `o || f()`, the object, not `true`",
    "file": "src/lower/mir/tidy/branches.rs",
    "find": "        (_, ExprKind::Bool(true)) => Expr::bin(Op::Or, truth(negate(test)), yes),\n",
    "replace": "        (_, ExprKind::Bool(true)) => Expr::bin(Op::Or, negate(test), yes),\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a truthy option's is_some given as a value"
    ]
  },
  {
    "name": "block-returns-kept",
    "breaks": "returns MIR jumps out of a block for stay `result = x; break l;` and one `return result;`",
    "file": "src/lower/mir/tidy/branches.rs",
    "find": "    returned_from_block(stmts, result);\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "written returns MIR jumps out of a block"
    ]
  },
  {
    "name": "or-branches-reversed",
    "breaks": "`if a || b { x } else { y }` from MIR is `!a && !b ? y : x`",
    "file": "src/lower/mir/tidy/branches.rs",
    "find": "                    true => conditional(test.clone().negated_test(), no.clone(), yes.clone(), booleans),\n",
    "replace": "                    true => conditional(test.clone(), yes.clone(), no.clone(), booleans),\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an if of an or from MIR keeps its branches"
    ]
  },
  {
    "name": "mir-tidy-or-chain-negated",
    "breaks": "an `||` chain from MIR is `!(!a && b !== c) || d`, its test negated whole",
    "file": "src/lower/mir/tidy/branches.rs",
    "find": "        ExprKind::Binary(Op::And | Op::Or, ..) => negated_bools(test, booleans),\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "an or chain"
    ]
  },
  {
    "name": "mir-tidy-bool-field-unknown",
    "breaks": "a `bool` field from MIR isn't known as one, `!!(element.editable || ..)`",
    "file": "src/lower/mir/tidy/branches.rs",
    "find": "        ExprKind::Member(..) => field_path(e).is_some_and(|path| booleans.contains(&path)),\n",
    "replace": "",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "or chain from MIR is written as one"
    ]
  },
  {
    "name": "mir-tidy-chain-first",
    "breaks": "a match's arms as values from MIR chain with the rest first, `variant !== \"foundation\" ? .. : [\"Foundation\", ..]`",
    "file": "src/lower/mir/tidy/branches.rs",
    "find": "                    (false, true) => false,\n",
    "replace": "                    (false, true) => true,\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "arms as values chain in written order"
    ]
  },
  {
    "name": "mir-tidy-or-pattern-blocks",
    "breaks": "an or-pattern's matches! from MIR tests its first pattern alone",
    "file": "src/lower/mir/tidy/branches.rs",
    "find": "                StmtKind::If(or_all(a, b), then, None)\n",
    "replace": "                StmtKind::If(a, then, None)\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "or-pattern's matches is an or"
    ]
  },
  {
    "name": "mir-tidy-same-test-truth",
    "breaks": "`!!error` isn't the same test as `error`, so a value made where an object's truth holds stays a `const`",
    "file": "src/lower/mir/tidy/branches.rs",
    "find": "    let (a, b) = (truth(a), truth(b));\n",
    "replace": "",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-tidy-bool-call-unknown",
    "breaks": "a call giving a `bool` isn't one in a test, `!!isItem(item) && ..`",
    "file": "src/lower/mir/tidy/branches.rs",
    "find": "        ExprKind::Call(..) if call_path(e).is_some_and(|path| booleans.contains(&path)) => true,\n",
    "replace": "",
    "tests": ["test/lowering.test.ts", "-t", "call giving a bool is one in a test"]
  },
  {
    "name": "logical-returns-kept",
    "breaks": "a value written with `&&`, `s != \"en\" && (if t {..} else {..})`, stays MIR's `return` of each branch",
    "file": "src/lower/mir/tidy/branches.rs",
    "find": "        && within(last_if.span)\n",
    "replace": "        && false\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a value written with && is returned as one"
    ]
  },
  {
    "name": "element-branch-not-small",
    "breaks": "a small element chosen, `busy ? <span>\u2026</span> : <b>Save</b>`, stays `let label; if (busy) ..`",
    "file": "src/lower/mir/tidy/branches.rs",
    "find": "        ExprKind::Jsx(_) => levels(e) <= 2,\n",
    "replace": "        ExprKind::Jsx(_) => false,\n",
    "tests": [
      "test/jsx.test.ts"
    ]
  },
  {
    "name": "mir-chain-tail-folded",
    "breaks": "a variable's value of three arms from MIR has its last two folded, `else { cls = w ? \"warn\" : \"tip\"; }`",
    "file": "src/lower/mir/tidy/branches.rs",
    "find": "                    && let Some((y, folded)) = assigned_only(otherwise)\n",
    "replace": "                    && let Some((y, folded)) = assigned_only(otherwise).filter(|_| false)\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a variable's value of three arms from MIR"
    ]
  },
  {
    "name": "mir-temp-chain-half-folded",
    "breaks": "a value MIR holds of three branches, its first doing more, is `else { tmp = b ? q : r; }`",
    "file": "src/lower/mir/tidy/branches.rs",
    "find": " || is_temporary(x) && then.len() > 1)",
    "replace": ")",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a variable's value of three arms from MIR"
    ]
  },
  {
    "name": "mir-tidy-truth-read-negated",
    "breaks": "a loop's last `if (size)` of an `Option` never falsy, `!!size`, is read as negated and becomes `if (!size) { continue; }`, the rest dedented",
    "file": "src/lower/mir/tidy/branches.rs",
    "find": "        ExprKind::Unary(UnaryOp::Not, c) => !matches!(c.kind, ExprKind::Unary(UnaryOp::Not, _)),\n",
    "replace": "        ExprKind::Unary(UnaryOp::Not, _c) => true,\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a let a loop's branch makes is declared in the loop"
    ]
  },
  {
    "name": "mir-tidy-table-taken-apart-unread",
    "breaks": "a `match` reading a table, taken apart, `let Look { name, classes } = match ..`, stays a conditional of every variant from MIR, not `LOOKS[kind]`",
    "file": "src/lower/mir/tidy/branches.rs",
    "find": "            | StmtKind::Return(Some(e))\n            | StmtKind::Destructure { value: e, .. } => {\n",
    "replace": "            | StmtKind::Return(Some(e)) => {\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a match reading a table from MIR"
    ]
  },
  {
    name: "mir-tidy-updates-folded",
    breaks: "MIR's branches each adding to a variable are `x = c ? x + a : x + b`",
    file: "src/lower/mir/tidy/branches.rs",
    find: "                    && !(updates(x, yes) && updates(x, no))\n",
    replace: "                    && !(updates(x, yes) && updates(x, no) && false)\n",
    tests: ["test/lowering.test.ts","-t","branches that each add"],
  },
  {
    name: "mir-tidy-if-condition-logical-folded",
    breaks: "MIR's `if` of an `||` as the function's value is `return a || b ? x : y`",
    file: "src/lower/mir/tidy/branches.rs",
    find: "    let condition = |l: &js::Span| conditions.iter().any(|(c, g)| c == g && c.lo <= l.lo && l.hi <= c.hi);\n",
    replace: "    let condition = |l: &js::Span| l.lo > l.hi;\n",
    tests: ["test/lowering.test.ts","-t","an if of a logical returns"],
  },
  {
    name: "mir-tidy-result-folded",
    breaks: "MIR's tail `if` making the function's value is one conditional, not returned in each",
    file: "src/lower/mir/tidy/branches.rs",
    find: "                    && !(x == result && written_if(span)) =>\n",
    replace: "                    =>\n",
    tests: ["test/lowering.test.ts","-t","a tail if returns each branch"],
  },
  {
    name: "mir-tidy-table-last-arm-unchecked",
    breaks: "MIR's last arm of a match on an enum's names is taken as its own name, whatever it gives",
    file: "src/lower/mir/tidy/branches.rs",
    find: "            None => Some(left.as_str()),\n",
    replace: "            None => None,\n",
    tests: ["test/lowering.test.ts","-t","an enum's own names are the enum"],
  },
  {
    name: "turned-of-floats",
    breaks: "`!(x < 50)` of a float is `x >= 50`, which a `NaN` makes `false` where Rust's test is `true`",
    file: "src/lower/mir/tidy/branches.rs",
    find: "        ExprKind::Var(v) => numbers.contains(v),\n",
    replace: "        ExprKind::Var(_) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "loop_exits"],
  },
  {
    name: "turned-not-of-text",
    breaks: "`!(c >= \"a\")` of a `char` stays so, not `c < \"a\"`",
    file: "src/lower/mir/tidy/branches.rs",
    find: "    let text = |x: &Expr| matches!(x.kind, ExprKind::Str(_));\n",
    replace: "    let text = |_: &Expr| false;\n",
    tests: ["test/snapshots.test.ts", "-t", "lexer"],
  },
];
