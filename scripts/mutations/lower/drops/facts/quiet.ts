// Mutations of src/lower/drops/facts/quiet.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "quiet-ignores-calls",
    breaks: "a call of the crate's beside a guard is taken as asking nothing",
    file: "src/lower/drops/facts/quiet.rs",
    find: "(self.borrows).call_asks(recognition, (self.owner, id, generic_args), receiver, self.cell, drops)",
    replace: "false",
    tests: [
      "test/lowering-cells.test.ts",
      "-t",
      "a field's RefCell only borrowed for a moment is its value",
    ],
  },
  {
    name: "quiet-ignores-drops",
    breaks: "a temporary's `Drop` beside a guard is taken as asking nothing",
    file: "src/lower/drops/facts/quiet.rs",
    find: "        if asks || writes_over || (!is_place(&expr.kind) && self.drops(expr.ty)) {",
    replace: "        if asks || writes_over {",
    tests: [
      "test/lowering-cells.test.ts",
      "-t",
      "a field's RefCell only borrowed for a moment is its value",
    ],
  },
  {
    name: "let-of-moved-drop-unseen",
    breaks:
      "a `let` of a value with a destructor moved from a place, `let q = p;`, is taken to run nothing, though it drops while a guard is held (ADR 0863)",
    file: "src/lower/drops/facts/quiet.rs",
    find: "            && self.drops(pattern.ty)\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_let_guard_drops_moved.rs$"],
  },
  {
    name: "write-over-drop-unseen",
    breaks:
      "a write over a value with a destructor, `p = q;`, is taken to run nothing, though its destructor runs while a guard is held (ADR 0863)",
    file: "src/lower/drops/facts/quiet.rs",
    find: "        let writes_over = matches!(expr.kind, ExprKind::Assign { lhs, .. } if self.drops(self.thir[lhs].ty));\n",
    replace: "        let writes_over = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_let_guard_writes_over.rs$"],
  },
  {
    name: "pointer-call-beside-guard-asks-nothing",
    breaks:
      "a fn pointer's call while a guard is held asks nothing of its cell, so `report`'s borrow isn't counted while `f`, which may be `touch`, borrows it mutably, and nothing panics",
    file: "src/lower/drops/facts/quiet.rs",
    find: "            None => self.borrows.pointer_call_asks(recognition, ptr, self.cell, drops),\n",
    replace: "            None => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^fn_pointer_borrows_held.rs$"],
  },
  {
    name: "unmet-of-a-float",
    breaks:
      "a `contains` of a float guards the push as a string's does, though `NaN` isn't equal to itself: the push's borrow of the same cell isn't checked, and runs where Rust's is busy (ADR 0863, amended)",
    file: "src/lower/drops/facts/quiet.rs",
    find: "        let reflexive = recognition.is_string_like(element) || element.is_integral() || element.is_bool();\n",
    replace: "        let reflexive = true;\n",
    tests: ["test/corpus.test.ts", "-t", "^contains_guarded_float.rs$"],
  },
  {
    name: "unmet-of-any-value",
    breaks:
      "a `contains` of any value guards the push, not only of the loop's item, which is there where the two are one cell (ADR 0863, amended)",
    file: "src/lower/drops/facts/quiet.rs",
    find: "        if !self.items.contains(&item) || !reflexive {\n",
    replace: "        if !reflexive {\n",
    tests: ["test/corpus.test.ts", "-t", "^contains_guarded_other.rs$"],
  },
  {
    name: "unmet-never",
    breaks:
      "React's scheduleGesture's push only where a type isn't there yet is counted, `$borrowMut`, as any borrow that may meet the loop's is",
    file: "src/lower/drops/facts/quiet.rs",
    find: "            && let Some(var) = self.unmet_in(cond)\n",
    replace:
      "            && let Some(var) = self.unmet_in(cond).filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^contains_guarded_borrow.rs$"],
    snapshots: true,
  },
  {
    name: "unmet-of-a-handle-set-again",
    breaks:
      "a handle the body sets again is one cell wherever it's read, so its borrow after the test may be of another, `own`'s",
    file: "src/lower/drops/facts/quiet.rs",
    find: "        (!changed).then_some(var)\n",
    replace: "        Some(var)\n",
    tests: ["test/corpus.test.ts", "-t", "^contains_guarded_set_again.rs$"],
  },
  {
    name: "held-borrow-direct-unclassed",
    breaks:
      "a borrow of another class's cell while a guard is held, `to.items.borrow_mut()` while `from.items` is walked, is taken as meeting it, counted (rd482)",
    file: "src/lower/drops/facts/quiet.rs",
    find: "                    && theirs.class.is_some()\n                    && let Some(called)",
    replace: "                    && theirs.class.is_some()\n                    && false\n                    && let Some(called)",
    tests: ["test/corpus.test.ts", "-t", "^held_borrow_of_own_cell.rs$"],
    snapshots: true,
  },
  {
    name: "ended-before-all-held",
    breaks:
      "what ran before a guard was made is taken as running while it's held, so every borrow of a cell given to a `println!`'s earlier argument is counted (#552)",
    file: "src/lower/drops/facts/quiet.rs",
    find: "        if !self.quiet || ended(self.ids, self.ended, expr) {\n",
    replace: "        if !self.quiet {\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_borrow_after_a_call.rs$"],
    snapshots: true,
  },
];
