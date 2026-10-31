// Mutations of src/lower/mir/queries.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-promoted-const-unseen",
    breaks:
      "a promoted `&HEADERS` is the constant's whole value, written where it's read, not `HEADERS[0]`",
    file: "src/lower/mir/queries.rs",
    find: "    (Some(local) == referred).then_some(c)\n",
    replace: "    (Some(local) == referred).then_some(c).filter(|_| false)\n",
    tests: ["test/compiler.test.ts", "-t", "the report's JS: function values"],
  },
  {
    name: "mir-early-exit-not-negated",
    breaks:
      "a branch that returns, written first, is taken when its test fails, not when it holds",
    file: "src/lower/mir/queries.rs",
    find: "            let mut out = vec![StmtKind::If(test.negated_test(), otherwise, None).at(span)];\n",
    replace:
      "            let mut out = vec![StmtKind::If(test, otherwise, None).at(span)];\n",
    tests: ["test/lowering-branches.test.ts", "-t", "branches from MIR read"],
  },
  {
    name: "mir-static-read-movable",
    breaks:
      "a read of a static, which any call may change, is moved past a call that changes it",
    file: "src/lower/mir/queries.rs",
    find: "                stable &= !state.naming.borrowed_names().contains(name) && state.own_names.contains(name);\n",
    replace:
      "                stable &= !state.naming.borrowed_names().contains(name);\n",
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
    name: "from-end-place-not-fixed",
    breaks:
      "an item counted from the end, given through a `&mut`, `zero(last)`, is `const at = v.length - 1` and `v[at]`, not its place, `v[v.length - 1]`",
    file: "src/lower/mir/queries.rs",
    find: "                && (index.is_constant() || matches!(index.kind, js::ExprKind::Var(_)) || from_end(object, index))\n",
    replace:
      "                && (index.is_constant() || matches!(index.kind, js::ExprKind::Var(_)))\n",
    tests: ["test/corpus.test.ts", "-t", "^slice_pattern_mut_rest.rs$"],
    snapshots: true,
  },
  {
    name: "from-end-place-not-same",
    breaks:
      "an item counted from the end through a `&mut`, `[.., last]`, is a handle, not its place `v[v.length - 1]`",
    file: "src/lower/mir/queries.rs",
    find: "                    || from_end(object, index)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^slice_pattern_mut_rest.rs$"],
    snapshots: true,
  },
  {
    name: "mir-early-exit-panic-last",
    breaks:
      "a check that panics is written after what goes on, `if (n === 6) { return n; } fail()`, not `if (n !== 6) { fail() } return n;`",
    file: "src/lower/mir/queries.rs",
    find: "        Some(otherwise) if panics == [false, true] => {\n",
    replace:
      "        Some(otherwise) if panics == [false, true] && false => {\n",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "what reads the same anywhere",
    ],
  },
  {
    name: "mir-panics-through-calls-not",
    breaks:
      "a panic whose message a call makes isn't seen as one, its check written last",
    file: "src/lower/mir/queries.rs",
    find: "            TerminatorKind::Goto { target }\n            | TerminatorKind::Call {\n                target: Some(target), ..\n            }\n",
    replace: "            TerminatorKind::Goto { target }\n",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "what reads the same anywhere",
    ],
  },
  {
    name: "mir-early-exit-both-returning-flipped",
    breaks:
      "of two branches that both return, the second is first, `if (!c) { return b; } return a;`",
    file: "src/lower/mir/queries.rs",
    find: "        Some(otherwise) if panics == [true, false] || exits(&then) => {\n",
    replace:
      "        Some(otherwise) if panics == [true, false] || exits(&then) && !exits(&otherwise) => {\n",
    tests: ["test/lowering-matches.test.ts", "-t", "let-else from MIR"],
  },
  {
    name: "mir-bounds-check-inline",
    breaks:
      "an index nothing proves in bounds is checked by an `if … throw` of its own, not `$index(xs, i)`",
    file: "src/lower/mir/queries.rs",
    find: "        _ if written.projection.last() == Some(&PlaceElem::Index(local)) => Some((local, true)),\n        _ => None,\n",
    replace: "        _ => None,\n",
    tests: ["test/lowering-numbers.test.ts", "-t", "nothing proves in bounds"],
  },
  {
    name: "mir-bounds-ref-unchecked-here",
    breaks:
      "a shared reference to an element, `&xs[i]`, is checked by an `if … throw`",
    file: "src/lower/mir/queries.rs",
    find: "        | Rvalue::Ref(_, BorrowKind::Shared, read)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(leak|loop_drops).rs$"],
    snapshots: true,
  },
  {
    name: "mir-capture-copied-never",
    breaks:
      "a closure made in a loop that changes what it captures reads the variable, every closure the last value",
    file: "src/lower/mir/queries.rs",
    find: "        next.extend(data.terminator().successors());\n",
    replace: "",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "captures a variable itself where nothing",
    ],
  },
  {
    name: "mir-division-checked-twice",
    breaks:
      "a `%` or `/` from MIR is checked before the division checks it, `if ((2 === -1) & \u2026) throw`",
    file: "src/lower/mir/queries.rs",
    find: "            AssertKind::DivisionByZero(_)\n                | AssertKind::RemainderByZero(_)\n                | AssertKind::Overflow(BinOp::Div | BinOp::Rem, ..)\n",
    replace: "            AssertKind::Overflow(BinOp::Shl, ..)\n",
    tests: [
      "test/lowering-numbers.test.ts",
      "-t",
      "division by a constant from MIR",
    ],
  },
  {
    name: "mir-frozen-borrow-a-variable",
    breaks:
      "a match guard's shared borrow of `k.level` makes `k` a variable, its payloads held, `const t = k._0; return t;`",
    file: "src/lower/mir/queries.rs",
    find: "&& !place.is_indirect() && !frozen {",
    replace: "&& !place.is_indirect() {",
    tests: [
      "test/lowering-matches.test.ts",
      "-t",
      "a match guard's binding from MIR",
    ],
  },
  {
    name: "mir-arm-written-at-test",
    breaks:
      "an arm that tests a field first, `Done { ok: true }`, is placed where its test is spanned, the `match`, first",
    file: "src/lower/mir/queries.rs",
    find: "    written_within(body, block, 4)\n",
    replace: "    written_within(body, block, 0)\n",
    tests: [
      "test/lowering-matches.test.ts",
      "-t",
      "a match's arms from MIR are in written order",
    ],
  },
  {
    name: "mir-async-captured-refused",
    breaks:
      "MIR's `async fn` that captures its array parameter first is refused",
    file: "src/lower/mir/queries.rs",
    find: "        .all(|s| matches!(&s.kind, StmtKind::Const(_, value) if value.reads_only_vars() || copy(value)))\n",
    replace:
      "        .all(|s| matches!(&s.kind, StmtKind::Const(_, value) if value.reads_only_vars()))\n",
    tests: [
      "test/lowering-arrows.test.ts",
      "-t",
      "an async fn that reads its array past an await",
    ],
  },
  {
    name: "constant-index-checked",
    breaks:
      "an array's constant index below its constant length is checked, `$index(grid, 1)`",
    file: "src/lower/mir/queries.rs",
    find: "                BinOp::Lt => Some(a < b),\n",
    replace: "                BinOp::Lt => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^array_repeat.rs$"],
    snapshots: true,
  },
  {
    name: "constant-divisor-checked",
    breaks:
      "a constant divisor is checked against 0 where a panic would drop, `if (2 === 0)`",
    file: "src/lower/mir/queries.rs",
    find: "                BinOp::Eq => Some(a == b),\n",
    replace: "                BinOp::Eq => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_operand_leaves.rs$"],
    snapshots: true,
  },
  {
    name: "constant-division-overflow-checked",
    breaks:
      "a signed division by a constant other than -1 is checked for overflow where a panic would drop",
    file: "src/lower/mir/queries.rs",
    find: "                (Some(false), _) | (_, Some(false)) => Some(false),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^drop_operand_leaves.rs$"],
    snapshots: true,
  },
  {
    name: "division-check-across-a-call",
    breaks:
      "a division's check is left to its helper though a call runs between, `v[f()] = a % b`: `f` runs before a zero divisor panics",
    file: "src/lower/mir/queries.rs",
    find: "            TerminatorKind::Goto { target } => block = *target,\n            _ => return false,\n",
    replace:
      "            TerminatorKind::Goto { target } => block = *target,\n            _ => return true,\n",
    tests: ["test/corpus.test.ts", "-t", "division_check_order"],
  },
  {
    name: "pin-borrow-a-write",
    breaks:
      "a `&mut` to a pin counts as a write of it, so its binding is a handle",
    file: "src/lower/mir/queries.rs",
    find: " && place.projection.is_empty() && !itself {",
    replace: " && place.projection.is_empty() {",
    tests: ["test/corpus.test.ts", "-t", "pin_box"],
    snapshots: true,
  },
  {
    name: "mir-some-value-unsteady",
    breaks:
      "MIR refuses `ref mut x` of a boxed `Some`'s payload, `$someValue(o)`",
    file: "src/lower/mir/queries.rs",
    find: "        js::ExprKind::Call(..) => crate::lower::std_types::option::some_value_of(place).is_some_and(steady_place),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "ref_mut_of_mut_payload"],
    snapshots: true,
  },
  {
    name: "reassigned-param-alike",
    breaks:
      "a `mut` parameter assigned once is read as what never changes, its argument not counted as its first value, so a read of it an operand takes before it's assigned, `n + { n += 1; n }`'s `n`, reads its new value, `n + n`",
    file: "src/lower/mir/queries.rs",
    find: "    written_again(state, local)\n        && !(state.locals.user[local]",
    replace:
      "    (state.locals.writes[local] > 1)\n        && !(state.locals.user[local]",
    tests: ["test/corpus.test.ts", "-t", "^mut_param_moved_out.rs$"],
  },
  {
    name: "mir-alike-once-each-way-unseen",
    breaks:
      "a variable each of a `match`'s arms assigns, `let variant = match ..`, isn't alike, so JSX holds `const Icon = variant.Icon` first, not `variant.Icon && <variant.Icon />`",
    file: "src/lower/mir/queries.rs",
    find: "        && !(state.locals.user[local] && !is_parameter(state, local) && once_each_way(state.body, local))\n",
    replace: "        && true\n",
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
    name: "mir-alike-machine-once",
    breaks:
      "a local MIR assigns twice on separate ways, a `vec!`'s box, reads alike, and what's around it is held in `const`s first",
    file: "src/lower/mir/queries.rs",
    find: "        && !(state.locals.user[local] && !is_parameter(state, local) && once_each_way(state.body, local))\n",
    replace:
      "        && !(!is_parameter(state, local) && once_each_way(state.body, local))\n",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "a vec.s specs from MIR stay in place",
    ],
  },
  {
    name: "macro-fn-starts-at-the-macro",
    breaks:
      "a function a macro writes starts where the macro does, so a comment in it before the `fn` is dropped",
    file: "src/lower/mir/queries.rs",
    find: "        Some(name) if whole.from_expansion() && !name.from_expansion() => {\n",
    replace: "        Some(name) if false => {\n",
    tests: [
      "test/lowering-comments.test.ts",
      "-t",
      "a comment in a macro before its fn",
    ],
  },
  {
    name: "written-break-not-first",
    breaks:
      "of an `if` whose branches each leave, a `return` and a `break` the program wrote, the `return`'s is first, its test negated (ADR 0790)",
    file: "src/lower/mir/queries.rs",
    find: "        }) => !span.is_none(),\n",
    replace: "        }) => false,\n",
    tests: [
      "test/port-performance-track.test.ts",
      "-t",
      "addValueToProperties is a switch whose case breaks where it's written",
    ],
  },
  {
    name: "read-before-written-past-a-write",
    breaks:
      "`node.v.set({ node = b; .. })` sets `b`'s, not the node `node` was where the `Cell` was reached",
    file: "src/lower/mir/queries.rs",
    find: "                && a.0.local == local\n                && a.0.projection.is_empty()\n            {\n                return false;\n",
    replace:
      "                && a.0.local == local\n                && a.0.projection.is_empty()\n            {\n                continue;\n",
    tests: ["test/corpus.test.ts", "-t", "^given_last_unwinds.rs$"],
  },
  {
    name: "index-parameter-not-given-once",
    breaks:
      "an element at a parameter's index, `root.lanes.borrow_mut()[index] |= lane`, is a getter and a setter of it, the parameter counted as written never",
    file: "src/lower/mir/queries.rs",
    find: "    state.locals.writes[local] + usize::from(is_parameter(state, local)) == 1\n}\n",
    replace: "    state.locals.writes[local] == 1\n}\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_vec_element_places.rs$"],
    snapshots: true,
  },
  {
    name: "const-index-unsettled",
    breaks:
      "a place at a `const` index isn't settled, so its value is made in a `const value` first, no compound (ADR 0894, amended)",
    file: "src/lower/mir/queries.rs",
    find: "                && (index.is_fixed() || matches!(&index.kind, js::ExprKind::Var(name) if settled(name)))\n",
    replace:
      "                && (index.is_constant() || matches!(&index.kind, js::ExprKind::Var(name) if settled(name)))\n",
    tests: [
      "test/lowering-bits.test.ts",
      "-t",
      "an item.s bitwise assignment is its compound",
    ],
  },
  {
    name: "read-from-past-assignment",
    breaks:
      "a read of a local past where it's assigned again, a loop's `&mut found` made anew before it's read, is taken for one of the value written, so `found.push(v)` holds it first, `const f = found; f.push(v)` (#198)",
    file: "src/lower/mir/queries.rs",
    find: "                Some(false) => continue 'blocks,\n",
    replace: "                Some(false) => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^if_let_mut_else_assign.rs$"],
    snapshots: true,
  },
  {
    name: "written-before-read-blind",
    breaks:
      "a binding of a `mut` parameter written before it's read, `shape = ..; n`, is read in place, its parameter held whole before it's written, `const shape$1 = shape;`, not as the binding it is (#121)",
    file: "src/lower/mir/queries.rs",
    find: "    uses.writes.iter().any(|write| {\n",
    replace: "    false && uses.writes.iter().any(|write| {\n",
    tests: ["test/corpus.test.ts", "-t", "^settled_until_written.rs$"],
    snapshots: true,
  },
  {
    name: "written-before-read-anywhere-after",
    breaks:
      "a parameter written after every read of a binding of it, after its `match`, keeps a `const n = shape._0;` in each arm (#121)",
    file: "src/lower/mir/queries.rs",
    find: "(after || reached.contains(&write.block)) && reaching.contains(&write.block)",
    replace: "(after || reached.contains(&write.block))",
    tests: ["test/corpus.test.ts", "-t", "^settled_until_written.rs$"],
    snapshots: true,
  },
  {
    name: "guard-goto-of-nowhere-followed",
    breaks:
      "a `goto` rustc places nowhere, a `continue`'s back to its loop's start, is followed as a guard's failure is, and React's `if (exitStatus !== ROOT_ERRORED) { continue; }` is turned, its `else` first",
    file: "src/lower/mir/queries.rs",
    find: "TerminatorKind::Goto { target } if only_goes_on && !here.is_dummy() => block = target,",
    replace:
      "TerminatorKind::Goto { target } if only_goes_on => block = target,",
    tests: ["test/corpus.test.ts", "-t", "^written_breaks_beside_an_else.rs$"],
    snapshots: true,
  },
  {
    name: "variant-read-taken-for-written",
    breaks:
      'a block that reads an enum\'s variant to test it is taken for written at the `match`\'s subject, before the arms, and a failed guard\'s way on goes first: `if (!(size(s) > 10)) { .. return "other"; } return "big";`',
    file: "src/lower/mir/queries.rs",
    find: "StatementKind::Assign(assign) if matches!(assign.1, Rvalue::Discriminant(_)))",
    replace: "StatementKind::Assign(assign) if matches!(assign.1, Rvalue::Discriminant(_)) && false)",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(guard_statements|nested_options|tuple_scrutinee_calls).rs$",
    ],
    snapshots: true,
  },
  {
    name: "guard-failure-at-its-goto",
    breaks:
      'a guard that fails, `_ if name == "HTML" => 2`, is taken for written where MIR spans its `goto`, before the arm it goes on to, and the arms are turned, `if (name !== "HTML") { return 3; } return 2;`',
    file: "src/lower/mir/queries.rs",
    find: "    for _ in 0..8 {\n",
    replace: "    for _ in 0..0 {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(untagged_enums|guard_statements|interpreter).rs$",
    ],
    snapshots: true,
  },
];
