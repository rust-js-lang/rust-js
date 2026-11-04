// Mutations of src/lower/mir/tidy/values.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "whole-writes-outside-closures",
    breaks:
      "a closure's whole write through a flattened state's handle isn't seen, so the handle is its place and the write `chunk = ..` (#482)",
    file: "src/lower/mir/tidy/values.rs",
    find: "    let mut written_whole = std::collections::HashSet::new();\n    js::each_block_mut(body, &mut |stmts| {\n",
    replace: "    let mut written_whole = std::collections::HashSet::new();\n    js::each_list(body, &mut |stmts| {\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_closure_writes.rs$"],
  },
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
    find: "    let plain: HashSet<&String> = (cells.iter()).filter(|c| reads.get(*c) == kept.get(*c)).collect();\n",
    replace: "    let plain: HashSet<&String> = cells.iter().collect();\n",
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
    name: "mir-tidy-handle-read-early",
    breaks:
      "MIR's handle on a `Cell` field is read where it's made, though set before it's read",
    file: "src/lower/mir/tidy/values.rs",
    find: "                let at_once = frozen.contains(name) || read_at_once(stmts, k, name);\n",
    replace: "                let at_once = true;\n",
    tests: ["test/lowering-cells.test.ts", "-t", "a Cell a let takes apart"],
  },
  {
    name: "mir-tidy-frozen-handle-at-its-uses",
    breaks:
      "a `let` of a `RefCell` field's `Ref` held across a call is read at each use, `store.chunks.get(key)`, its name lost (rd658)",
    file: "src/lower/mir/tidy/values.rs",
    find: "                let at_once = frozen.contains(name) || read_at_once(stmts, k, name);\n",
    replace: "                let at_once = read_at_once(stmts, k, name);\n",
    tests: ["test/corpus.test.ts", "-t", "^frozen_borrow_named.rs$"],
    snapshots: true,
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
    find: '            ..Expr::call(Expr::member(items.clone(), "join"), vec![Expr::str("")])\n        };\n    }\n',
    replace:
      '            ..Expr::call(Expr::member(items.clone(), "join"), vec![Expr::str("")])\n        };\n        return;\n    }\n',
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
    name: "read-handle-written-at-once",
    breaks:
      "a handle written through in a closure, `r.set(5)` of `let r = &context.value`, is read where it's made, `const r = context.value; .. r = 5`",
    file: "src/lower/mir/tidy/values.rs",
    find: "        *at_once &= !changed.contains(name);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^cell_alias_in_a_closure.rs$"],
  },
  {
    name: "read-handle-write-unstored",
    breaks:
      "a nullable field's `Cell` set in a closure through a variable writes `undefined`, the handle's storage left out",
    file: "src/lower/mir/tidy/values.rs",
    find: "                    *value = storage.written(std::mem::replace(value, Expr::undefined()), false);\n",
    replace: "                    let _ = storage;\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "through a reference and a helper",
    ],
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
    find: "    defined.then(|| var.clone())\n",
    replace:
      "    Some(var.clone()).filter(|_| defined || matches!(op, Op::Eq))\n",
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
    name: "read-handle-this-rooted-kept",
    breaks:
      "a `&Cell` handle on a field of `this` read later is kept, not its place, `const reads = { get value() { .. } }` (#359)",
    file: "src/lower/mir/tidy/values.rs",
    find: "        let rereads = matches!(root.kind, ExprKind::Var(_) | ExprKind::This);\n",
    replace: "        let rereads = matches!(root.kind, ExprKind::Var(_));\n",
    tests: ["test/classes.test.ts", "-t", "Cell fields"],
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
    name: "in-place-handle-assigned",
    breaks: "a flattened state written whole through its handle assigns the variable holding its object, `chunk = { status }`, not each key (rd380)",
    file: "src/lower/mir/tidy/values.rs",
    find: "                && let Some(sets) = storage.written_in_place(",
    replace: "                && false\n                && let Some(sets) = storage.written_in_place(",
    tests: ["test/classes-flattened-state.test.ts", "-t", "made, read and replaced in place"],
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
    name: "in-place-handle-kept-whole",
    breaks:
      "a flattened state's handle written whole through stays an object of a getter and a setter, `const borrowMut = { get value() .., set value(value) { chunk.status = .. } }`, where each write is the chunk's keys (#478)",
    file: "src/lower/mir/tidy/values.rs",
    find: "    if !whole.is_empty() {\n",
    replace: "    if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_let_borrow.rs$"],
    snapshots: true,
  },
  {
    name: "handle-read-once-held",
    breaks:
      "a `match` arm's binding of a `Cell` field read once is held first, `const current = kind.current; return current;`, not `return kind.current;` (#532)",
    file: "src/lower/mir/tidy/values.rs",
    find: "        if *at_once && rereads && !written && js::mentions_in(body, name) == 1 {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_binding_read_once.rs$"],
    snapshots: true,
  },
  {
    name: "handle-of-a-let-read-in-place",
    breaks:
      "a `let` the program wrote of a cell field, `let Fiber { tag, type_, .. } = fiber;`, read once, loses its name, `fiber.tag` where it's read (#532)",
    file: "src/lower/mir/tidy/values.rs",
    find: "        if *at_once && rereads && !written && js::mentions_in(body, name) == 1 {\n",
    replace: "        if *at_once && rereads && js::mentions_in(body, name) == 1 {\n",
    tests: ["test/lowering-field-cells.test.ts", "-t", "lets side by side are consts as written"],
  },
  {
    name: "late-field-handles-unread",
    breaks: "a handle on a field declared only once its `let` sinks to its branch, a guard's of a binding's `*tries.borrow_mut() += 1`, stays a handle, `const borrowMut = { get value() .. }` (#503)",
    file: "src/lower/mir/tidy/values.rs",
    find: "                && (matches!(place.kind, ExprKind::Member(..))\n                    && !(in_place && matches!(storage, js::Storage::InPlace(_)))\n",
    replace: "                && (matches!(place.kind, ExprKind::Member(..))\n                    && !in_place\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_variant_fields.rs$"],
    snapshots: true,
  },
  {
    name: "nullable-handle-of-variable-kept",
    breaks:
      "a variable's handle on a nullable parameter or an item isn't read as its place, `const place = { get value() { return x; } .. }` (#541)",
    file: "src/lower/mir/tidy/values.rs",
    find: "                    || matches!(storage, js::Storage::NullForNone)\n",
    replace: "                    || false && matches!(storage, js::Storage::NullForNone)\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_written_through_borrows.rs$"],
    snapshots: true,
  },
  {
    name: "handle-of-an-item-not-reread",
    breaks:
      "a handle on an item, `fiber.lanes[0]`, isn't read again where it's used, so it's kept, `const place = { get value() { return fiber.lanes[0]; } .. }` (#541)",
    file: "src/lower/mir/tidy/values.rs",
    find: "        while let ExprKind::Member(of, _) | ExprKind::Index(of, _) = &root.kind {\n",
    replace: "        while let ExprKind::Member(of, _) = &root.kind {\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_written_through_borrows.rs$"],
    snapshots: true,
  },
  {
    name: "handle-on-a-state-handles-value-kept",
    breaks:
      "`Cell::from_mut(&mut *state)` of a flattened state's guard given to a function, or `&mut *state` given to one that gives it back, is a handle on the class's object, `set value(value) { task = value; }`, which assigns the variable, not each key (#659)",
    file: "src/lower/mir/tidy/values.rs",
    find: "            && matches!(&of.kind, ExprKind::Var(v) if states.contains(v))\n",
    replace: "            && matches!(&of.kind, ExprKind::Var(v) if states.contains(v) && v.is_empty())\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_handed_on.rs$"],
  },
  {
    name: "in-place-spills-share-a-name",
    breaks:
      "two whole writes through a flattened state's handle in one list each spill what they write as `state`, declared twice (#687)",
    file: "src/lower/mir/tidy/values.rs",
    find: "                        let fresh = crate::names::fresh_in(&mut taken, \"state\");\n",
    replace: "                        let fresh = String::from(\"state\");\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_cell_swapped.rs$"],
  },
];
