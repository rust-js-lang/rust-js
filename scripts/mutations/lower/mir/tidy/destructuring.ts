// Mutations of src/lower/mir/tidy/destructuring.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "assigned-parts-left-apart",
    breaks:
      "a tuple assigned to variables, `(a, b) = e`, reads `const [_18, _19] = e; a = _18; b = _19;`, not `[a, b] = e;` (#331)",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "        if !distinct || parts.iter().any(|part| js::mentions_in(&stmts[after..], part) > 0) {\n",
    replace: "        if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^tuple_assigned_to_variables.rs$"],
    snapshots: true,
  },

  {
    name: "named-let-part-of-a-pattern",
    breaks:
      "`let payload = &lazy.payload; let init = lazy.init;` is `const { _payload: payload, _init: init } = lazy;`",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "        if !made && let Some(k) = parts.iter().position(|(_, bound, ..)| lets.contains(bound)) {\n",
    replace:
      "        if false && let Some(k) = parts.iter().position(|(_, bound, ..)| lets.contains(bound)) {\n",
    tests: ["test/lowering-parts.test.ts", "-t", "a let of a part"],
  },
  {
    name: "rust-js-names-in-a-pattern",
    breaks:
      "`Cell::new(update.lane.get())`'s read ahead of the next field's is `const { lane: value, tag: value$1 } = update;`, names the program never gave (ADR 0598, amended)",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "        if !made && let Some(k) = parts.iter().position(|(_, bound, ..)| !user.contains(bound)) {\n",
    replace:
      "        if false && let Some(k) = parts.iter().position(|(_, bound, ..)| !user.contains(bound)) {\n",
    tests: [
      "test/lowering-field-cells.test.ts",
      "-t",
      "a struct literal of cells another's read makes reads each in place",
    ],
  },
  {
    name: "mir-tidy-destructured-kept",
    breaks:
      "`const _5 = e; const a = _5[0]; const b = _5[1];` stays, not `const [a, b] = e;`",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "        let enough = count >= if made || bound { 1 } else { 2 };\n",
    replace: "        let enough = !made && false;\n",
    tests: [
      "test/lowering-matches.test.ts",
      "-t",
      "bindings from MIR are destructured",
    ],
  },
  {
    name: "mir-tidy-destructured-variable-kept",
    breaks:
      "`const a = t[0]; const b = t[1];` of a variable stays, not `const [a, b] = t;`",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "                (v.to_string(), i)\n",
    replace: "                (v.to_string(), i + stmts.len())\n",
    tests: [
      "test/lowering-matches.test.ts",
      "-t",
      "bindings from MIR are destructured",
    ],
  },
  {
    name: "mir-tidy-array-parts-kept",
    breaks:
      "`let (root, files) = (chosen.root.clone(), ..)` isn't split here, and `root` is read as `const root = chosen.root;`, not taken from `chosen` as `const { root } = chosen;`",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "            stmts.splice(i..first + count, each);\n            continue;\n",
    replace:
      "            let _ = each;\n            i += 1;\n            continue;\n",
    tests: ["test/snapshots.test.ts", "-t", "playground"],
  },
  {
    name: "mir-tidy-params-not-destructured",
    breaks:
      "`(_2) => { const i = _2[0]; const c = _2[1]; … }` stays, not `([i, c]) => …`",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "        body.drain(start..start + count);\n        *param = pattern;\n",
    replace: "        let _ = pattern;\n",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "iterator chains from MIR|a lazy chain from MIR spread once",
    ],
  },
  {
    name: "mir-tidy-default-part-unstarted",
    breaks:
      "parts read with a default don't start a pattern, `const className = p.k === undefined ? d : p.k;` stays",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "        ExprKind::Cond(_, _, read) => read,\n",
    replace: "        ExprKind::Cond(_, _, read) if false => read,\n",
    tests: [
      "test/lowering-bindings.test.ts",
      "-t",
      "props field's default is read",
    ],
  },
  {
    name: "mir-tidy-deref-param-kept",
    breaks: "`|&n|` stays `(_2) => { const n = _2; … }`",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "            *param = js::Pattern::Name(name.clone());\n            body.remove(0);\n",
    replace: "            break;\n",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "names its .&x. parameter",
    ],
  },
  {
    name: "async-arg-named",
    breaks:
      "`__arg0`, an `async fn`'s parameter, is a name the program wrote, kept",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "        is_temporary(name) || arg.is_some_and(",
    replace: "        is_temporary(name) || false && arg.is_some_and(",
    tests: [
      "test/lowering-matches.test.ts",
      "-t",
      "an async fn's parameter pattern from MIR",
    ],
  },
  {
    name: "one-field-pattern-kept-apart",
    breaks:
      "a `let` pattern of one field of a named variable is `const message = error.message;`",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "let enough = count >= if made || bound { 1 } else { 2 };",
    replace: "let enough = count >= if made { 1 } else { 2 };",
    tests: [
      "test/lowering-matches.test.ts",
      "-t",
      "a let pattern of a named variable takes it apart",
    ],
  },
  {
    name: "destructured-param-kept",
    breaks:
      "an `async fn`'s parameter taken apart in its body stays `(__arg0) { const { params } = __arg0;`",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "            *param = pattern.clone();\n            body.remove(0);\n            continue;\n",
    replace: "            let _ = pattern;\n",
    tests: [
      "test/lowering-matches.test.ts",
      "-t",
      "an async fn's parameter pattern from MIR",
    ],
  },
  {
    name: "mir-tidy-tuple-set-through",
    breaks:
      "a tuple MIR sets an item of, `t[0] = x`, is read as its items, which crashes printing what's assigned",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "        if writes.replaced.contains(&t)\n            || writes.changed.contains(&t)\n            || indexed == 0\n",
    replace: "        if indexed == 0\n",
    tests: ["test/corpus.test.ts", "-t", "mut_ref_temporary"],
  },
  {
    name: "nested-pattern-flat-only",
    breaks:
      "a closure's pattern of a pattern stays `(_2) => { const name = _2[1].name; .. }`, not `([n, { name }]) => ..`",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "        _ if group.iter().all(|(rest, ..)| !rest.is_empty()) => Some((nested_pattern(group)?, None)),",
    replace: "        _ if false => Some((nested_pattern(group)?, None)),",
    tests: [
      "test/lowering-matches.test.ts",
      "-t",
      "a closure's nested patterns are nested JS patterns",
    ],
  },
  {
    name: "mir-tidy-part-named-twice",
    breaks:
      "MIR's two names of one part are taken apart as one, `const [b] = x`, the first lost",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "        if parts.iter().any(|(p, ..)| *p == part) {\n            break;\n        }\n",
    replace: "",
    tests: ["test/lowering-names.test.ts", "-t", "two names of one field"],
  },
  {
    name: "mir-tidy-async-param-copied",
    breaks:
      "MIR's `async fn` copies its parameter where its body starts, `let n$1 = n`",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "        && js::mentions_in(body, param) == 1\n",
    replace: "        && false\n",
    tests: [
      "test/compiler.test.ts",
      "-t",
      "async code becomes async functions and await",
    ],
  },
  {
    name: "tuple-acting-parts-unheld",
    breaks:
      "a tuple matched whose part calls, `(v.pop(), n > 3)`, calls it where each part is read, more than once",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "            if last.is_some_and(|last| k < last && !fixed(item) || k == last) {\n",
    replace:
      "            if false && last.is_some_and(|last| k < last && !fixed(item) || k == last) {\n",
    tests: ["test/corpus.test.ts", "-t", "tuple_scrutinee_calls"],
  },
  {
    name: "tuple-earlier-reads-unheld",
    breaks: "`v.len()` of `(v.len(), v.pop())` is read after the pop",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "            if last.is_some_and(|last| k < last && !fixed(item) || k == last) {\n",
    replace: "            if last.is_some_and(|last| k == last) {\n",
    tests: ["test/corpus.test.ts", "-t", "tuple_scrutinee_calls"],
  },
  {
    name: "tuple-part-member-fixed",
    breaks:
      "a property read before a call, `v.length`, is taken for what no call changes",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "                    ExprKind::Member(of, _) => math(of),\n",
    replace: "                    ExprKind::Member(..) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "tuple_scrutinee_calls"],
  },
  {
    name: "tuple-part-set-after",
    breaks:
      "a variable a guard sets after the tuple is made is read as the guard left it",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "if !assigns(after, v) && !set_elsewhere.contains(v))",
    replace: "if !set_elsewhere.contains(v))",
    tests: ["test/corpus.test.ts", "-t", "tuple_scrutinee_calls"],
  },
  {
    name: "tuple-part-set-by-closure",
    breaks:
      "a variable a closure sets after the tuple is made is read as the closure left it",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "if !assigns(after, v) && !set_elsewhere.contains(v))",
    replace: "if !assigns(after, v))",
    tests: ["test/corpus.test.ts", "-t", "tuple_scrutinee_calls"],
  },
  {
    name: "constants-first-unmoved",
    breaks:
      "a `let`'s parts with a drop flag between, `const b = items[1]; let d$live = true; const d = items[3];`, aren't one pattern",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "        let moved = constants_first(stmts, i, first, &name);\n",
    replace: "        let moved = 0;\n",
    tests: ["test/corpus.test.ts", "-t", "^array_partial_moves.rs$"],
    snapshots: true,
  },
  {
    name: "parameter-constants-unmoved",
    breaks:
      "a parameter's parts with drop flags between, `([a, b, c, d], i)`, are `_1`, read part by part",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "        constants_first(body, 0, 0, &name);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^array_partial_moves.rs$"],
    snapshots: true,
  },
  {
    name: "parameter-constants-unskipped",
    breaks:
      "a parameter's parts after its drop flags aren't its pattern: it's `_1`",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "                |stmt| matches!(&stmt.kind, StmtKind::Let(bound, Some(value)) if value.is_constant() && *bound != name),\n",
    replace: "                |_| false,\n",
    tests: ["test/corpus.test.ts", "-t", "^array_partial_moves.rs$"],
    snapshots: true,
  },
  {
    name: "constants-first-trailing",
    breaks:
      "a constant's `let` after a value's parts, or after a temporary with none, moves before it",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "    while constants.last().map(|&k| k + 1) == Some(end) {\n",
    replace: "    while false {\n",
    tests: ["test/corpus.test.ts", "-t", "^array_partial_moves.rs$"],
    snapshots: true,
  },
  {
    name: "conditional-tuples-kept-whole",
    breaks:
      "`let (a, b) = if c { (x, y) } else { (z, w) };` is `const [a, b] = c ? [x, y] : [z, w];`, two arrays made, where upstream's trackHydrated assigns two variables in each branch (rd45)",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "            && matches!(value.kind, ExprKind::Cond(..))\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^tuple_of_an_if_taken_apart.rs$"],
    snapshots: true,
  },
  {
    name: "conditional-tuples-else-dropped",
    breaks:
      "a tuple of an `if`'s value taken apart assigns its parts in the `then` branch alone, so the `else`'s values are never given (a wrong answer, rd45)",
    file: "src/lower/mir/tidy/destructuring.rs",
    find: "                StmtKind::If((**test).clone(), then, Some(otherwise)).at(value.span),\n",
    replace:
      "                StmtKind::If((**test).clone(), then, None).at(value.span),\n",
    tests: ["test/corpus.test.ts", "-t", "^tuple_of_an_if_taken_apart.rs$"],
  },
];
