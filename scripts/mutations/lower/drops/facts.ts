// Mutations of src/lower/drops/facts.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "drop-let-value-context",
    breaks:
      "a `let`'s value is taken as its block's, so a closure that holds a value with a destructor, made for a `let`, is refused",
    file: "src/lower/drops/facts.rs",
    find: "            if self.lets.contains_key(&child) {\n                return (None, child);\n            }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "closure_drops"],
  },
  {
    name: "temp-taken-apart-refused",
    breaks:
      "`let (x, _) = (make(3), make(4));` in a trait's default method is refused",
    file: "src/lower/drops/facts.rs",
    find: "                if !self.follows(once(pat)) {",
    replace: "                if true {",
    tests: ["test/corpus.test.ts", "-t", "default_method_drops"],
  },
  {
    name: "closure-made-anywhere",
    breaks:
      "a closure that holds a value with a destructor is made where its drop can't see what it took",
    file: "src/lower/drops/facts.rs",
    find: "                    if !placed && !carried {",
    replace: "                    if false && !placed && !carried {",
    tests: ["test/traits.test.ts", "-t", "closure made in a struct"],
  },
  {
    name: "fn-closure-given-away",
    breaks:
      "an `Fn` closure that holds a value with a destructor is given to a call that consumes it, and never drops it",
    file: "src/lower/drops/facts.rs",
    find: "        matches!(ty.kind(), ty::Closure(_, args) if args.as_closure().kind() != ty::ClosureKind::FnOnce)",
    replace: "        false",
    tests: [
      "test/bindings.test.ts",
      "-t",
      "listener holding a value with a destructor",
    ],
  },
  {
    name: "closure-assigned",
    breaks:
      "a closure that holds a value with a destructor is assigned where its drop can't see what it took",
    file: "src/lower/drops/facts.rs",
    find: '                self.closure_problem(expr.span, "assigning a closure that holds a value with a destructor");',
    replace: "                let _ = expr;",
    tests: ["test/traits.test.ts", "-t", "closure assigned"],
  },
  {
    name: "momentary-always",
    breaks:
      "a guard held while its statement calls code that borrows is only checked",
    file: "src/lower/drops/facts.rs",
    find: "        if self.held_quietly((fun, cell), (&[], &extents), &self.ended_before()) {\n",
    replace:
      "        if self.held_quietly((fun, cell), (&[], &extents), &self.ended_before()) || true {\n",
    tests: [
      "test/lowering-cells.test.ts",
      "-t",
      "a field's RefCell only borrowed for a moment is its value",
    ],
  },
  {
    name: "momentary-never",
    breaks: "every guard is held, where nothing can ask",
    file: "src/lower/drops/facts.rs",
    find: "        if used_in_place {\n            self.momentary(e);\n        }\n",
    replace: "",
    tests: [
      "test/jsx-react-dev.test.ts",
      "-t",
      "react.dev's InlineToc nests its headings in a field's array",
    ],
  },
  {
    name: "quiet-ignores-calls",
    breaks: "a call of the crate's beside a guard is taken as asking nothing",
    file: "src/lower/drops/facts.rs",
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
    file: "src/lower/drops/facts.rs",
    find: "        if asks || writes_over || (!is_place(&expr.kind) && self.drops(expr.ty)) {",
    replace: "        if asks || writes_over {",
    tests: [
      "test/lowering-cells.test.ts",
      "-t",
      "a field's RefCell only borrowed for a moment is its value",
    ],
  },
  {
    name: "quiet-statements-unseen",
    breaks: "a guard a statement holds is held, where nothing can ask",
    file: "src/lower/drops/facts.rs",
    find: "                    .find(|(s, _)| *s == scope)\n",
    replace: "                    .find(|(s, _)| *s == scope && false)\n",
    tests: [
      "test/lowering-cells.test.ts",
      "-t",
      "a field's RefCell only borrowed for a moment is its value",
    ],
  },
  {
    name: "std-given-closure-refused",
    breaks:
      "a closure called by `&` given by value to std, which drops it as it returns, is refused",
    file: "src/lower/drops/facts.rs",
    find: "            && (def_id.is_local() || crate::lower::recognition::is_std_crate(tcx, def_id))\n",
    replace: "            && def_id.is_local()\n",
    tests: ["test/corpus.test.ts", "-t", "^given_function_drops.rs$"],
  },
  {
    name: "returned-closure-made-refused",
    breaks:
      "a closure its maker gives back, holding a value with a destructor, is refused where it's made",
    file: "src/lower/drops/facts.rs",
    find: "                    if !placed && !carried {\n",
    replace: "                    if !placed {\n",
    tests: ["test/corpus.test.ts", "-t", "^returned_closure_drops.rs$"],
  },
  {
    name: "transparent-made-here",
    breaks:
      "a closure that holds a value with a destructor, in an `AssertUnwindSafe` given to `catch_unwind`, is refused as made where it's not kept",
    file: "src/lower/drops/facts.rs",
    find: "                ExprKind::Adt(..) if self.cx.recognition.transparent(self.thir[parent].ty).is_some() => child = parent,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^catch_unwind_assert_drops.rs$"],
  },
  {
    name: "closure-block-value-refused",
    breaks:
      "a closure holding an `Rc` made as a block's value for a `let` is refused",
    file: "src/lower/drops/facts.rs",
    find: '                            self.block_for_let(parent.expect("a block"))\n',
    replace: "                            false\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_block_value.rs$"],
  },
  {
    name: "arm-temporaries-unscoped",
    breaks:
      "a borrow in a `match` arm's value, `Some(prev) => prev.types.borrow().clone()`, is taken as held past the arm, counted, and its cell `{ value }`",
    file: "src/lower/drops/facts.rs",
    find: "            .push((arm.scope, arm.guard.is_none().then_some(arm.body)));\n",
    replace: "            .push((arm.scope, None));\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_quiet_borrows.rs$"],
    snapshots: true,
  },
  {
    name: "if-let-scrutinee-unscoped",
    breaks:
      "a borrow an `if let` tests is taken as held where nothing says how long, so a consumer's of a library's cell is refused",
    file: "src/lower/drops/facts.rs",
    find: "            } if if_then_scope == scope => Some([cond, then]),\n",
    replace:
      "            } if if_then_scope == scope && false => Some([cond, then]),\n",
    tests: ["test/crates.test.ts", "-t", "two crates: ref_cells"],
  },
  {
    name: "if-let-else-held",
    breaks:
      "an `if let`'s scrutinee's borrow is taken to end before its `then`, so one held while the `then` borrows its cell runs on where Rust panics",
    file: "src/lower/drops/facts.rs",
    find: "            (Some(both), _) => both.to_vec(),\n",
    replace: "            (Some(both), _) => both[..1].to_vec(),\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_if_let_held.rs$"],
  },
  {
    name: "held-always-seen",
    breaks:
      "a borrow held while JS's code runs is taken as held while only what rust-js sees runs, so a consumer holds one of a cell a library borrows unchecked (ADR 0863)",
    file: "src/lower/drops/facts.rs",
    find: "        if seen.seen {\n            self.facts.held_while_seen.insert(fun);\n",
    replace:
      "        if true {\n            self.facts.held_while_seen.insert(fun);\n",
    tests: ["test/crates.test.ts", "-t", "a library's RefCells are the values"],
  },
  {
    name: "let-held-never",
    breaks:
      "a guard a `let` holds while nothing can ask is counted, so its cell's every borrow is checked, `const v = $borrow(log, true)` where it's `const v = log` (ADR 0863)",
    file: "src/lower/drops/facts.rs",
    find: "            self.facts.let_held.insert(fun);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^refcell_let_guard_quiet.rs$"],
    snapshots: true,
  },
  {
    name: "let-held-moved-ignored",
    breaks:
      "a `let`'s guard moved out of its block, `kept = v;`, is taken as held only until its block ends, so a borrow it meets after doesn't panic (ADR 0863)",
    file: "src/lower/drops/facts.rs",
    find: "        if !moves.moved && self.held_quietly((fun, cell), (rest, value.as_slice()), &HashSet::new()) {\n",
    replace:
      "        if self.held_quietly((fun, cell), (rest, value.as_slice()), &HashSet::new()) {\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_let_guard_kept.rs$"],
  },
  {
    name: "let-held-past-drop",
    breaks:
      "a `let`'s guard dropped by `drop(v);` is taken as held until its block ends, so what borrows its cell after counts it (ADR 0863)",
    file: "src/lower/drops/facts.rs",
    find: "            Some(at) => (&rest[..at], None),\n",
    replace: "            Some(_) => (rest, value),\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_let_guard_quiet.rs$"],
    snapshots: true,
  },
  {
    name: "let-of-moved-drop-unseen",
    breaks:
      "a `let` of a value with a destructor moved from a place, `let q = p;`, is taken to run nothing, though it drops while a guard is held (ADR 0863)",
    file: "src/lower/drops/facts.rs",
    find: "            && self.drops(pattern.ty)\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_let_guard_drops_moved.rs$"],
  },
  {
    name: "write-over-drop-unseen",
    breaks:
      "a write over a value with a destructor, `p = q;`, is taken to run nothing, though its destructor runs while a guard is held (ADR 0863)",
    file: "src/lower/drops/facts.rs",
    find: "        let writes_over = matches!(expr.kind, ExprKind::Assign { lhs, .. } if self.drops(self.thir[lhs].ty));\n",
    replace: "        let writes_over = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_let_guard_writes_over.rs$"],
  },
  {
    name: "pointer-call-beside-guard-asks-nothing",
    breaks:
      "a fn pointer's call while a guard is held asks nothing of its cell, so `report`'s borrow isn't counted while `f`, which may be `touch`, borrows it mutably, and nothing panics",
    file: "src/lower/drops/facts.rs",
    find: "            None => self.borrows.pointer_call_asks(recognition, ptr, self.cell, drops),\n",
    replace: "            None => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^fn_pointer_borrows_held.rs$"],
  },
  {
    name: "let-else-guard-held-through-else",
    breaks:
      "a let-else's scrutinee's guard is taken to live through its `else`, so React's `update.next.borrow().clone()` counts the field's every borrow, `$cellReplace($borrowMut($fieldCell(update, \"next\", true)), ..)`",
    file: "src/lower/drops/facts.rs",
    find: "                initializer,\n                ..\n            } => (init_scope, initializer),\n",
    replace:
      "                initializer,\n                else_block,\n                ..\n            } => (init_scope, initializer.filter(|_| else_block.is_none())),\n",
    tests: ["test/corpus.test.ts", "-t", "^let_else_guard_released.rs$"],
    snapshots: true,
  },
  {
    name: "unmet-of-a-float",
    breaks:
      "a `contains` of a float guards the push as a string's does, though `NaN` isn't equal to itself: the push's borrow of the same cell isn't checked, and runs where Rust's is busy (ADR 0863, amended)",
    file: "src/lower/drops/facts.rs",
    find: "        let reflexive = recognition.is_string_like(element) || element.is_integral() || element.is_bool();\n",
    replace: "        let reflexive = true;\n",
    tests: ["test/corpus.test.ts", "-t", "^contains_guarded_float.rs$"],
  },
  {
    name: "unmet-of-any-value",
    breaks:
      "a `contains` of any value guards the push, not only of the loop's item, which is there where the two are one cell (ADR 0863, amended)",
    file: "src/lower/drops/facts.rs",
    find: "        if !self.items.contains(&item) || !reflexive {\n",
    replace: "        if !reflexive {\n",
    tests: ["test/corpus.test.ts", "-t", "^contains_guarded_other.rs$"],
  },
  {
    name: "unmet-never",
    breaks:
      "React's scheduleGesture's push only where a type isn't there yet is counted, `$borrowMut`, as any borrow that may meet the loop's is",
    file: "src/lower/drops/facts.rs",
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
    file: "src/lower/drops/facts.rs",
    find: "        (!changed).then_some(var)\n",
    replace: "        Some(var)\n",
    tests: ["test/corpus.test.ts", "-t", "^contains_guarded_set_again.rs$"],
  },
  {
    name: "assigned-value-held",
    breaks:
      "a guard of an assignment's place, `*c.borrow_mut() = f(v);`, is taken as held while its value is evaluated, which Rust does first, so a library's `f` refuses it (ADR 0863, gap rd32)",
    file: "src/lower/drops/facts.rs",
    find: "                vec![lhs]\n",
    replace: "                vec![extent]\n",
    tests: [
      "test/crates.test.ts",
      "-t",
      "two crates: assigned_after_its_value",
    ],
  },
  {
    name: "called-once-closure-refused",
    breaks:
      "a closure rustc infers `Fn` that holds a value with a destructor, given where it's made to a binding that calls it once, is refused as one given away without being called once (ADR 1020)",
    file: "src/lower/drops/facts.rs",
    find: "                            && !matches!(*ty.kind(), ty::Closure(id, _) if self.cx.closure_facts.called_once(id))\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^called_once_closure_drops_after.rs$",
    ],
  },
  {
    name: "receiver-guard-held-through-arguments",
    breaks:
      "a guard that's only a call's receiver is held while the call's other arguments run, so `target.borrow_mut().push(show(value))` counts the array",
    file: "src/lower/drops/facts.rs",
    find: "                    && (*receiver == e || self.stack.contains(receiver))\n",
    replace: "                    && false\n",
    tests: ["test/lowering-cell-borrows.test.ts", "-t", "only a call's receiver"],
  },
  {
    name: "receiver-guard-ignores-arguments",
    breaks:
      "a borrow of the receiver's cell the program wrote in an argument, `c.borrow_mut().push(c.borrow().len())`, is taken as meeting nothing",
    file: "src/lower/drops/facts.rs",
    find: "        for &arg in rest {\n            quiet.visit_expr(&self.thir[arg]);\n        }\n",
    replace: "",
    tests: ["test/lowering-cell-borrows.test.ts", "-t", "only a call's receiver"],
  },
  {
    name: "argument-guard-held-through-binding",
    breaks:
      "a guard read only for a binding's argument is held while the binding runs, so `prepare(response.nonce.borrow().as_deref())` counts the cell, `$borrow` and a `try`/`finally` (rd427)",
    file: "src/lower/drops/facts.rs",
    find: "                    && self.given_of_guard(args[at], e) =>\n",
    replace: "                    && self.given_of_guard(args[at], e) && false =>\n",
    tests: ["test/lowering-cell-borrows.test.ts", "-t", "read only for a binding"],
  },
  {
    name: "argument-guard-of-any-call",
    breaks:
      "a guard read only for an argument of the crate's own function, which writes the cell, is taken as held only while it's read, so Rust's `already borrowed` never comes (rd427)",
    file: "src/lower/drops/facts.rs",
    find: "                        .is_some_and(|(id, _)| crate::lower::bindings::is_binding(recognition.tcx, id))\n",
    replace: "                        .is_some()\n",
    tests: ["test/corpus.test.ts", "-t", "^argument_guard_reentered.rs$"],
  },
  {
    name: "argument-guard-of-a-clone-only",
    breaks: "a guard's `as_deref()` or copy given to a binding is held while the binding runs, and counted (rd427)",
    file: "src/lower/drops/facts.rs",
    find: "            _ => lent(arg),\n",
    replace: "            _ => false,\n",
    tests: ["test/lowering-cell-borrows.test.ts", "-t", "read only for a binding"],
  },
  {
    name: "argument-guard-later-arguments-unasked",
    breaks:
      "an argument after a guard's read, `log_time(*t.borrow(), renamed(r))`, whose call writes the cell, is taken as running after the guard's given back, so Rust's `already borrowed` never comes (rd427)",
    file: "src/lower/drops/facts.rs",
    find: "                args[at..].to_vec()\n",
    replace: "                args[at..=at].to_vec()\n",
    tests: ["test/lowering-cell-borrows.test.ts", "-t", "read only for a binding"],
  },
  {
    name: "receiver-guard-ignores-call",
    breaks:
      "the call whose receiver holds a guard, `add(&mut m.borrow_mut(), m)` of an `add` that borrows `m`, is taken as asking nothing",
    file: "src/lower/drops/facts.rs",
    find: "        quiet.visit_call(call, method);\n",
    replace: "",
    tests: ["test/lowering-cell-borrows.test.ts", "-t", "only a call's receiver"],
  },
  {
    name: "receiver-guard-arguments-unseen",
    breaks:
      "a call's arguments run before its receiver's guard is held are asked as anything, so JS that may call back counts the array",
    file: "src/lower/drops/facts.rs",
    find: "        quiet.seen_only = true;\n",
    replace: "",
    tests: ["test/lowering-cell-borrows.test.ts", "-t", "only a call's receiver"],
  },
  {
    name: "held-borrow-direct-unclassed",
    breaks:
      "a borrow of another class's cell while a guard is held, `to.items.borrow_mut()` while `from.items` is walked, is taken as meeting it, counted (rd482)",
    file: "src/lower/drops/facts.rs",
    find: "                    && theirs.class.is_some() =>\n",
    replace: "                    && theirs.class.is_some()\n                    && false =>\n",
    tests: ["test/corpus.test.ts", "-t", "^held_borrow_of_own_cell.rs$"],
    snapshots: true,
  },
  {
    name: "let-held-through-unseen",
    breaks:
      "a `let` of a borrow of what a guard guards, `let state = &mut *chunk.state.borrow_mut();`, is never held quietly, so a flattened state borrowed so is refused (rd432)",
    file: "src/lower/drops/facts.rs",
    find: "        let through = self.guarded_borrow(init);\n",
    replace: "        let through: Option<ExprId> = None;\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_let_borrow.rs$"],
  },
  {
    name: "ended-before-arguments",
    breaks:
      "a guard made after a call's earlier argument returns, `sum(held(&log, 2), log.borrow().len())`, is taken as held while that argument runs, so its cell is counted (#552)",
    file: "src/lower/drops/facts.rs",
    find: "                        &args[..at]\n",
    replace: "                        &[]\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_borrow_after_a_call.rs$"],
    snapshots: true,
  },
  {
    name: "ended-before-callee",
    breaks:
      "a guard given to what a call returns, `picked(&log)(2, log.borrow().len())`, is taken as held while that call runs, so its cell is counted (#552)",
    file: "src/lower/drops/facts.rs",
    find: "                        ended.insert(fun);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^refcell_borrow_after_a_call.rs$"],
    snapshots: true,
  },
  {
    name: "ended-before-parts",
    breaks:
      "a guard made after a tuple's or an array's earlier part, `(held(&log, 3), log.borrow().len())`, is taken as held while that part runs, so its cell is counted (#552)",
    file: "src/lower/drops/facts.rs",
    find: "                        Some(at) => &fields[..at],\n",
    replace: "                        Some(_) => &[],\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_borrow_after_a_call.rs$"],
    snapshots: true,
  },
  {
    name: "ended-after-parts",
    breaks:
      "a tuple's parts after a guard are taken as ended before it's made, so the `RefMut` of `println!(\"{:?} {}\", log.borrow_mut(), log.borrow().len())` isn't counted, and the `borrow()` it meets doesn't panic (#552)",
    file: "src/lower/drops/facts.rs",
    find: "                        Some(at) => &fields[..at],\n",
    replace: "                        Some(at) => &fields[at + 1..],\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_borrow_after_a_held_one.rs$"],
  },
  {
    name: "ended-before-left-side",
    breaks:
      "a guard made on a binary operation's right side, `held(&log, 2) + log.borrow().len()`, is taken as held while its left side runs, so its cell is counted (#552)",
    file: "src/lower/drops/facts.rs",
    find: "                    ended.insert(lhs);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^refcell_borrow_after_a_call.rs$"],
    snapshots: true,
  },
  {
    name: "ended-before-all-held",
    breaks:
      "what ran before a guard was made is taken as running while it's held, so every borrow of a cell given to a `println!`'s earlier argument is counted (#552)",
    file: "src/lower/drops/facts.rs",
    find: "        if !self.quiet || ended(self.ids, self.ended, expr) {\n",
    replace: "        if !self.quiet {\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_borrow_after_a_call.rs$"],
    snapshots: true,
  },
  {
    name: "ended-before-all-unseen",
    breaks:
      "a library's code that returned before a guard was made is taken as running while it's held, so a consumer's borrow made after it is refused (#552)",
    file: "src/lower/drops/facts.rs",
    find: "        if !self.seen || ended(self.ids, self.ended, expr) {\n",
    replace: "        if !self.seen {\n",
    tests: ["test/crates.test.ts", "-t", "a library's RefCells are the values they hold"],
  },
];
