// Mutations of src/lower/analysis/cell_borrows/at_hand.rs (ADR 0093): what
// of the crate is a value at hand, whose property reads meet no getter of
// the program's (#2106, ADR 1173).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "at-hand-of-any-capture",
    breaks: "a property read of any variable runs nothing, a JS caller's `&Locale` parameter's too, whose getter may call back into the program while a borrow is held, where Rust panics `already borrowed` (#2106)",
    file: "src/lower/analysis/cell_borrows/at_hand.rs",
    find: "            ExprKind::UpvarRef { var_hir_id, .. } => self.vars.contains(var_hir_id),\n",
    replace: "            ExprKind::UpvarRef { .. } => true,\n",
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "a JS caller's locale"],
  },
  {
    name: "at-hand-of-any-binding",
    breaks: "a binding's result is at hand whether or not it makes it, `Map.get`'s of what a JS caller stored too (#2106)",
    file: "src/lower/analysis/cell_borrows/at_hand.rs",
    find: "                    matches!(js_form(tcx, callee), JsForm::New(_)) || creates(tcx, callee)\n",
    replace: "                    matches!(js_form(tcx, callee), JsForm::New(_)) || true\n",
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "what a map kept"],
  },
  {
    name: "creates-unread",
    breaks: "a binding that `creates` its result, `regExp.exec(s)`, isn't taken to, so a read of its match keeps the vercel-ui SDK's memo checked (#2106)",
    file: "src/lower/analysis/cell_borrows/at_hand.rs",
    find: "                    matches!(js_form(tcx, callee), JsForm::New(_)) || creates(tcx, callee)\n",
    replace: "                    matches!(js_form(tcx, callee), JsForm::New(_))\n",
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "the match exec made"],
  },
  {
    name: "at-hand-combinator-parameter-lost",
    breaks: "an `Option` combinator's closure parameter given an at-hand payload, `.and_then(|matched| matched.get(1))`, isn't at hand (#2106)",
    file: "src/lower/analysis/cell_borrows/at_hand.rs",
    find: "        if !combinator || !self.holds(recognition, thir, *subject) {\n",
    replace: "        if true {\n",
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "the match exec made"],
  },
  {
    name: "at-hand-of-any-variable",
    breaks: "a property read of any variable runs nothing, a JS caller's parameter read while its function holds a `RefCell` guard too, whose getter may call back and borrow it again (#2106)",
    file: "src/lower/analysis/cell_borrows/at_hand.rs",
    find: "            ExprKind::VarRef { id } => self.vars.contains(id),\n",
    replace: "            ExprKind::VarRef { .. } => true,\n",
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "a guard held over a JS caller's locale"],
  },
  {
    name: "at-hand-assignment-unread",
    breaks: "a variable a JS caller's value is assigned over one a builtin made is still taken to be at hand, its read under a held cell plain (#2110)",
    file: "src/lower/analysis/cell_borrows/at_hand.rs",
    find: "                        .all(|&(thir, value)| at_hand.holds(recognition, thir, value))\n",
    replace: "                        .all(|_| true)\n",
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "a JS caller's locale is assigned to"],
  },
  {
    name: "at-hand-loan-unread",
    breaks: "a variable lent `&mut` or bound `ref mut` is still taken to be at hand, though what's written through the loan isn't seen (#2110)",
    file: "src/lower/analysis/cell_borrows/at_hand.rs",
    find: "                !lent.contains(var)\n                    && ",
    replace: "                ",
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "lent mutably"],
  },
  {
    name: "at-hand-mut-borrow-not-lending",
    breaks: "a `&mut` borrow of a variable isn't taken to lend it, so `std::mem::swap(&mut locale, ..)` leaves it at hand (#2110)",
    file: "src/lower/analysis/cell_borrows/at_hand.rs",
    find: "                    borrow_kind: BorrowKind::Mut { .. },\n",
    replace: "                    borrow_kind: BorrowKind::Fake(_),\n",
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "lent mutably"],
  },
  {
    name: "at-hand-ref-mut-not-lending",
    breaks: "a `ref mut` binding isn't taken to lend what it binds into, so `*slot = tag` leaves the variable at hand (#2110)",
    file: "src/lower/analysis/cell_borrows/at_hand.rs",
    find: "                lends |= matches!(",
    replace: "                lends |= false && matches!(",
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "ref mut"],
  },
  {
    name: "at-hand-capture-assignment-unseen",
    breaks: "a closure's assignment to its own capture isn't seen, so a `move` closure reads a JS caller's locale it assigned as at hand (#2110)",
    file: "src/lower/analysis/cell_borrows/at_hand.rs",
    find: "        ExprKind::UpvarRef { var_hir_id, .. } => Some((*var_hir_id, true)),\n",
    replace: "        ExprKind::UpvarRef { .. } => None,\n",
    tests: ["test/builtins-runs-nothing.test.ts", "-t", "a closure assigns"],
  },
];
