// Mutations of src/js/borrow_checks.rs (ADR 0093, ADR 1141).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "borrow-check-some-where-unhoisted",
    breaks: "a borrow check in a `SomeWhere`'s test isn't hoisted, `$borrow(x).length === 0 && .. ? .. : undefined`",
    file: "src/js/borrow_checks.rs",
    find: "        ExprKind::Cond(test, ..) | ExprKind::SomeWhere(test, _) => (vec![&mut **test], true),\n",
    replace: "        ExprKind::Cond(test, ..) => (vec![&mut **test], true),\n",
    tests: ["test/borrow-forms.test.ts", "-t", "a then's test"],
  },
  {
    name: "momentary-shared-borrow-refused-beside-shared",
    breaks: "a momentary shared borrow is refused where another shared borrow is held, `RefCell already borrowed`, where Rust's lends it",
    file: "src/js/borrow_checks.rs",
    find: "            false => (Expr::bin(Op::Lt, borrows, Expr::int(0)), \"$alreadyMutablyBorrowed\"),\n",
    replace: "            false => (borrows, \"$alreadyMutablyBorrowed\"),\n",
    tests: ["test/corpus.test.ts", "-t", "^an_index_is_no_pattern.rs$"],
  },  {
    name: "momentary-hoisted-out-of-a-short-circuit",
    breaks: "a momentary borrow in a short circuit's right side is checked before it, `look && !cell.borrow().is_empty()` panicking where `look` is false and Rust borrows nothing",
    file: "src/js/borrow_checks.rs",
    find: "        ExprKind::Binary(Op::And | Op::Or | Op::Coalesce, left, _) => (vec![&mut **left], true),\n",
    replace: "",
    tests: ["test/borrow-forms.test.ts", "-t", "a loop's test or a branch"],
  },
  {
    name: "momentary-alias-kept",
    breaks: "a `const` that held a momentary borrow is kept once its check is first, `const borrowMut = nodes;`, where a person reads `nodes`",
    file: "src/js/borrow_checks.rs",
    find: "            if !declares(rest, &cell)\n",
    replace: "            if false && !declares(rest, &cell)\n",
    tests: ["test/borrow-forms.test.ts", "-t", "checked where they're called"],
  },
  {
    name: "momentary-alias-read-taken",
    breaks: "an alias read as its cell where a block after declares the cell's word, `f(nodes)` there reading the block's `nodes`, not the cell (#1681 change 3)",
    file: "src/js/borrow_checks.rs",
    find: "                && super::scopes::not_read_as_themselves(&read, &[&cell]).is_empty()\n",
    replace: "",
    unitTests: "js::borrow_checks::tests::an_alias_is_its_cell_but_where_a_block_declares_the_cells_word",
  },
  {
    name: "momentary-never-checked-first",
    breaks: "a momentary borrow evaluated first in its statement stays the call, `$borrowMut(nodes)`, a frame where React's replayElement's catch has room for none",
    file: "src/js/borrow_checks.rs",
    find: "        let checks = stmts[i].borrow_checks_first();\n",
    replace: "        let checks: Vec<Stmt> = Vec::new();\n",
    tests: ["test/borrow-forms.test.ts", "-t", "checked where they're called"],
  },
  {
    name: "guard-assigned-its-own-cell-kept",
    breaks: "a guard a `let` holds named as the parameter it borrows, checked alone, leaves `state = state;` (#1815)",
    file: "src/js/borrow_checks.rs",
    find: "        if itself {\n",
    replace: "        if false && itself {\n",
    tests: ["test/borrow-forms.test.ts", "-t", "a quiet let guard of a counted cell"],
  },
];
