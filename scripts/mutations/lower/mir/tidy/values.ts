// Mutations of src/lower/mir/tidy/values.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "field-path-not-of-this",
    breaks:
      "no path is read through `this`, so a class's method's read of its own `bool` field isn't known a `bool`: `!!this.open || this.below(x)`",
    file: "src/lower/mir/tidy/values.rs",
    find: '                ExprKind::This => "this".to_string(),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^class_bool_methods.rs$"],
    snapshots: true,
  },
  {
    name: "mir-tidy-plain-cell-escapes",
    breaks:
      "a cell given whole, `bump(c)`, is a variable too, and what it's given reads `undefined`",
    file: "src/lower/mir/tidy/values.rs",
    find: "    let plain: IndexSet<&String> = (cells.iter()).filter(|c| reads.get(*c) == kept.get(*c)).collect();\n",
    replace: "    let plain: IndexSet<&String> = cells.iter().collect();\n",
    tests: [
      "test/lowering-cells.test.ts",
      "-t",
      "only its function reads and sets",
    ],
  },
  {
    name: "array-spread-of-array-kept",
    breaks:
      "`[...first, ...[a, b]]` keeps its spread of an array, not `[...first, a, b]`",
    file: "src/lower/mir/tidy/values.rs",
    find: "    if let ExprKind::Array(items) = &mut e.kind {",
    replace: "    if let ExprKind::Array(items) = &mut e.kind && false {",
    tests: [
      "test/lowering-collections.test.ts",
      "-t",
      "arrays joined are one array of their items",
    ],
  },
  {
    name: "array-values-kept-stepped",
    breaks:
      "an array's `$iter` an adapter alone steps stays `$iter(xs).filter(..)`, not `xs.values().filter(..)`",
    file: "src/lower/mir/tidy/values.rs",
    find: "        && crate::lower::iterators::is_stepped_items(it)\n        && let ExprKind::Call(_, args) = &mut it.kind\n",
    replace:
      "        && false\n        && let ExprKind::Call(_, args) = &mut it.kind\n",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "an array's iterator for an adapter alone is its values",
    ],
  },
  {
    name: "named-callback-reads-base",
    breaks:
      "a closure reading what it would be named, `step` of `step$1`, is named it, reading itself",
    file: "src/lower/mir/tidy/values.rs",
    find: "            !reads\n",
    replace: "            true\n",
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
    name: "mir-enumerate-mapped",
    breaks:
      "`iter().enumerate()` of a slice from MIR is `xs.values().map((x, i) => [i, x])`, not `xs.entries()`",
    file: "src/lower/mir/tidy/values.rs",
    find: "        && pairs_index(pair)\n",
    replace: "        && false\n",
    tests: [
      "test/lowering-loops.test.ts",
      "-t",
      "nested loops from MIR read as written",
    ],
  },
  {
    name: "mir-rest-str-of-fresh",
    breaks:
      '`w.chars().as_str()` from MIR is `$restStr($iter(Array.from(w)))`, not `Array.from(w).join("")`',
    file: "src/lower/mir/tidy/values.rs",
    find: '        && callee.helper_name() == Some("$restStr")\n',
    replace: "        && false\n",
    tests: [
      "test/lowering-strings.test.ts",
      "-t",
      "the rest of a string's chars from MIR",
    ],
  },
  {
    name: "mir-capital-chars-joined",
    breaks:
      '`c.to_uppercase().collect::<String>()` from MIR is `Array.from(c.toUpperCase()).join("")`',
    file: "src/lower/mir/tidy/values.rs",
    find: "        && chars.facts.has(crate::js::Known::Chars)\n",
    replace: "        && false\n",
    tests: [
      "test/lowering-iterators.test.ts",
      "-t",
      "iterator chains from MIR are array methods",
    ],
  },
  {
    name: "mir-tidy-plain-cell-shadowed",
    breaks:
      "a cell a callback's parameter shadows is made plain, `(item) => item.value` read as `item`",
    file: "src/lower/mir/tidy/values.rs",
    find: "    cells.retain(|c| bound.get(c).copied().unwrap_or(0) <= 1);\n",
    replace: "",
    tests: [
      "test/lowering-cells.test.ts",
      "-t",
      "callback's parameter shadows",
    ],
  },
  {
    name: "lazy-flatmap-stepped",
    breaks: "a lazy flatten of arrays keeps `$iter(item)`",
    file: "src/lower/mir/tidy/values.rs",
    find: "        *each = Expr::var(param);\n",
    replace: "",
    tests: [
      "test/lowering-collections.test.ts",
      "-t",
      "arrays of arrays flattened",
    ],
  },
  {
    name: "applied-assignment-unseen",
    breaks:
      "`out += `${((r) => ..)(f())}``, an assignment's value, keeps its function called in place",
    file: "src/lower/mir/tidy/values.rs",
    find: "            StmtKind::Assign(target, e) | StmtKind::Compound(_, target, e) => {\n",
    replace:
      "            StmtKind::Assign(target, e) | StmtKind::Compound(_, target, e) if false => {\n",
    tests: ["test/snapshots.test.ts", "-t", "calc"],
    snapshots: true,
  },
  {
    name: "applied-arrow-kept",
    breaks:
      "a closure read in place is called where it's made, `((e) => e.data)(r._0)`, not its value",
    file: "src/lower/mir/tidy/values.rs",
    find: "        *e = Expr {\n            span: e.span,\n            ..applied\n        };\n",
    replace: "        let _ = applied;\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(poison_error|is_sorted_items|option_combinator_drops).rs$",
    ],
    snapshots: true,
  },
  {
    name: "rest-str-case-mapping-unjoined",
    breaks:
      '`{}` of `c.to_uppercase()` is `Array.from(c.toUpperCase()).join("")`, not `c.toUpperCase()`',
    file: "src/lower/mir/tidy/values.rs",
    find: "            ..Expr::builtin(items.clone(), \"join\", vec![Expr::str(\"\")])\n        };\n    }\n",
    replace: "            ..Expr::builtin(items.clone(), \"join\", vec![Expr::str(\"\")])\n        };\n        return;\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^writer_callables.rs$"],
    snapshots: true,
  },
  {
    name: "holed-lists-of-other-reads",
    breaks:
      "a list with holes read whole, or by another index, is its one item all the same",
    file: "src/lower/mir/tidy/values.rs",
    find: "        if js::mentions_in(&rest, &name) != usize::from(assigned)\n            || js::mentions_in(body, &name) == usize::from(assigned)\n        {\n",
    replace:
      "        if js::mentions_in(body, &name) == usize::from(assigned) {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(drop_functions|std_odds|associated_type_drops|and_value_drops).rs$",
    ],
  },
  {
    name: "checked-set-failure-dropped",
    breaks:
      "a check and a set merged, `xs[$at(xs, i)] = v`, lose the `VecDeque`'s message the check had",
    file: "src/lower/mir/tidy/values.rs",
    find: "            let at = [vec![items.clone(), index.clone()], failure.to_vec()].concat();",
    replace: "            let at = vec![items.clone(), index.clone()];",
    tests: ["test/corpus.test.ts", "-t", "^deque_write_out_of_bounds.rs$"],
  },
  {
    name: "nulled-kept-coalesced",
    breaks:
      "a variable that holds a nullable field's value is stored `x ?? null`",
    file: "src/lower/mir/tidy/values.rs",
    find: "            *e = (**made).clone().or_at(span);\n",
    replace: "            let _ = (made, span);\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "a nullable field's value a variable holds",
    ],
  },
  {
    name: "tested-defined-still-nulled",
    breaks:
      "`x !== undefined ? x ?? null : null` keeps its `?? null`, of a value the test shows defined (ADR 0620, amended)",
    file: "src/lower/mir/tidy/values.rs",
    find: "        Op::Ne => matches!(other.kind, ExprKind::Undefined | ExprKind::GivenUndefined),\n",
    replace: "        Op::Ne => false,\n",
    tests: [
      "test/lowering-written-branches.test.ts",
      "-t",
      "an if of returns the program wrote stays an if",
    ],
  },
  {
    name: "tested-defined-where-it-isnt",
    breaks:
      "`x === undefined` is taken to show `x` defined, its `?? null` dropped where it's `undefined`",
    file: "src/lower/mir/tidy/values.rs",
    find: "    defined.then(|| var.to_string())\n",
    replace:
      "    Some(var.to_string()).filter(|_| defined || matches!(op, Op::Eq))\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "a nullable result tested equal to a None given is null",
    ],
  },
  {
    name: "tested-defined-if-still-nulled",
    breaks:
      "`if (x !== undefined) { return x ?? null; }` keeps its `?? null`, of a value its test shows defined (ADR 0620, amended)",
    file: "src/lower/mir/tidy/values.rs",
    find: "                js::each_expr_mut(then, &mut |e| unnulled(e, &x));\n",
    replace: "                let _ = (&then, &x);\n",
    tests: [
      "test/lowering-written-branches.test.ts",
      "-t",
      "an if of returns the program wrote stays an if",
    ],
  },
  {
    name: "present-place-stored-coalesced",
    breaks:
      "React's `nextHydratableInstance = fiber.stateNode` after `fiber.stateNode != null` is `?? null` again (ADR 0957)",
    file: "src/lower/mir/tidy/values.rs",
    find: "                && shown_present(test, made)\n",
    replace:
      "                && shown_present(test, made)\n                && false\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "test shows present"],
  },
  {
    name: "present-place-of-an-or",
    breaks:
      "a part of an `||` taken as shown, `a || p != null` storing `p` as it is where `p` may be `undefined` (ADR 0957)",
    file: "src/lower/mir/tidy/values.rs",
    find: "        ExprKind::Binary(Op::And, a, b) => shown_present(a, place) || shown_present(b, place),\n",
    replace:
      "        ExprKind::Binary(Op::And | Op::Or, a, b) => shown_present(a, place) || shown_present(b, place),\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "test shows present"],
  },
  {
    name: "lets-never-undefined-unread",
    breaks:
      "React's `return parentFragmentInstances` of a `let` made `null` is `?? null` (ADR 0957, amended)",
    file: "src/lower/mir/tidy/values.rs",
    find: "    defined.extend(lets_never_undefined(body, &declared));\n",
    replace: "",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "set only to values"],
  },
  {
    name: "lets-never-undefined-of-closures",
    breaks:
      "a `let` a closure sets, maybe to `undefined`, is returned with no `?? null` (ADR 0957, amended)",
    file: "src/lower/mir/tidy/values.rs",
    find: "        ExprKind::Arrow(_, inner) | ExprKind::AsyncArrow(_, inner) => closed(inner),\n",
    replace: "",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "set only to values"],
  },
  {
    name: "unread-resets-off",
    breaks:
      "a take of a cell, returned, that no closure reads after it, writes its reset, `const old = result; result = \"\"; return old;`, where upstream's is `return result;` (rd598)",
    file: "src/lower/mir/tidy/values.rs",
    find: "    if lets.is_empty() {\n        return;\n    }\n",
    replace: "    if true {\n        return;\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^taken_and_returned.rs$"],
    snapshots: true,
  },
  {
    name: "unread-resets-past-closure-reads",
    breaks:
      "a take's reset is dropped where a closure kept past the take reads the cell, which then sees what was taken, not what the take left (rd598)",
    file: "src/lower/mir/tidy/values.rs",
    find: "            !handled.contains(name) && !finally_read.contains(name) && !read_by_closures(body, name, *appended)\n",
    replace: "            !handled.contains(name) && !finally_read.contains(name)\n",
    tests: ["test/corpus.test.ts", "-t", "^taken_and_returned.rs$"],
  },
  {
    name: "closures-left-in-tries",
    breaks: "a closure made between calls one `try` covers stays a `let` set in it, `map = (d) => ..`, not a function where its `let` is (#769)",
    file: "src/lower/mir/tidy/values.rs",
    find: "            stmts[i] = StmtKind::Function(Box::new(function)).at(span);\n            i += 1;\n",
    replace: "            i += 1;\n            let _ = function;\n",
    tests: ["test/corpus.test.ts", "-t", "^flatten_fold_closure_panics.rs$"],
    snapshots: true,
  },
  {
    name: "unread-map-assignment-kept",
    breaks: "MIR's `let` and assignment of a map nothing reads, a call's `try`'s, is kept, left declared after its `try` is gone (#805)",
    file: "src/lower/mir/tidy/values.rs",
    find: "    let gone: IndexSet<String> = (assigned.into_iter())\n        .filter(|name| js::mentions_in(body, name) == 1)\n        .collect();\n",
    replace: "    let gone: IndexSet<String> = IndexSet::default();\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^unread_map_of_a_held_iterator.rs$"],
  },
  {
    name: "chars-collected-kept",
    breaks: "a string's `chars()` made the runtime's and collected reads `Array.from($rev($chars(s)))`, not `Array.from(s).toReversed()`",
    file: "src/lower/mir/tidy/values.rs",
    find: "        if let Some(s) = crate::lower::iterators::chars_made_of(rev.unwrap_or(it)) {\n",
    replace: "        if let Some(s) = crate::lower::iterators::chars_made_of(rev.unwrap_or(it)).filter(|_| false) {\n",
    tests: ["test/compiler.test.ts", "-t", "iterators are array methods"],
  },
  {
    name: "lent-string-iterator-kept",
    breaks: "a lent string iterator made where it's read is `$lent(s[Symbol.iterator]())`, not itself",
    file: "src/lower/mir/tidy/values.rs",
    find: "    if let Some(it) = crate::lower::iterators::lent_string_iterator(e) {\n",
    replace: "    if let Some(it) = crate::lower::iterators::lent_string_iterator(e).filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^long_text_read_char_by_char.rs$"],
    snapshots: true,
  },
];
