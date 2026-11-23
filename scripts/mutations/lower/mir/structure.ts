// Mutations of crates/rust-js-mir/src/structure.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

const given1840 = ["test/none-given.test.ts", "-t", "in Bun and in Chromium"];

export const mutations: Mutation[] = [
  {
    name: "division-raised-at-its-assignment",
    breaks:
      "a division or remainder written to a place that's there already, `t.0 = t.1 / d`, `last = 17 % d`, panics naming where the assignment starts, not the division (#2366)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                        .then(|| super::queries::division_check_span(state.body, at))\n",
    replace: "                        .then(|| None::<Span>)\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(division_assigned_to_a_place_raises_at_the_division|remainder_assigned_in_a_closure_raises_at_the_remainder).rs$",
    ],
  },
  {
    name: "call-raised-at-its-whole-span",
    breaks: "a method call's panic names where its expression starts, not its method's name, as codegen's `fn_span`",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "            TerminatorKind::Call { fn_span, .. } => *fn_span,\n",
    replace: "            TerminatorKind::Call { .. } => terminator.source_info.span,\n",
    tests: ["test/binaries.test.ts", "-t", "program's own code"],
  },
  {
    name: "guard-box-not-copied-back-on-unwind",
    breaks: "`set_then_panic(&mut slot.borrow_mut())` in a `catch_unwind` loses what it wrote before it panicked, the guard taken for a place of the body's own (#1021)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "            && !is_guard(state.body.local_decls[local].ty)))\n",
    replace: "            && !(false && is_guard(state.body.local_decls[local].ty))))\n",
    tests: ["test/corpus.test.ts", "-t", "^written_through_a_guard_or_a_reference_to_an_object.rs$"],
  },
  {
    name: "alike-arms-tested",
    breaks:
      "an untagged enum's arms that each do the same JS still test the variant, `source instanceof Document ? source.firstChild : source.firstChild`, which throws a ReferenceError in a host without the `Document` global where Rust runs on (a wrong answer, rd687)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "        out.extend(alike.or(chain).unwrap_or_default());\n",
    replace: "        out.extend(chain.unwrap_or_default());\n",
    tests: ["test/webapi-objects.test.ts", "-t", "arms that each do the same"],
  },
  {
    name: "unalike-arms-taken",
    breaks:
      "an enum's arms are taken for one where they differ, a variant's `p._0.x` read of every variant, `p._1.x`'s too (a wrong answer, rd687)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "            alike = alike.filter(|first| js::stmts_same(first, &then)).map(|_| then.clone());\n",
    replace: "            alike = alike.map(|_| then.clone());\n",
    tests: ["test/corpus.test.ts", "-t", "^match_arms_alike.rs$"],
  },
  {
    name: "lone-class-branch-dropped",
    breaks:
      "an untagged enum's lone class, tested by the others' tests, is no branch: a form entry that's a `File` goes to the string's arm (a wrong answer, rd506's audit)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "            otherwise = Some(last).filter(|&b| reached(b));\n",
    replace: "",
    tests: ["test/webapi-objects.test.ts", "-t", "one class"],
  },
  {
    name: "self-handle-own-unnamed",
    breaks:
      "a closure's upgrade of its handle on itself is written nowhere, the local it gives read undeclared, not the closure's own name",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "            if let Some((own, name)) = state.facts.own(block) {\n                state.in_place.insert(own, name.clone());\n            }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^self_handle.rs$"],
  },
  {
    name: "self-handle-cell-written",
    breaks:
      "a handle's cell's statements are written though its calls aren't, `const this$ = ..` of nothing",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "            if state.facts.unlowered(at) {\n                out.extend(made);\n",
    replace:
      "            if false && state.facts.unlowered(at) {\n                out.extend(made);\n",
    tests: ["test/corpus.test.ts", "-t", "^self_handle.rs$"],
    snapshots: true,
  },
  {
    name: "settle-before-branches-skipped",
    breaks:
      "a value read in place before a `match` whose one arm drops is assigned in that arm alone, and the others compare `undefined`: `*e > match .. { .. }` (#274)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "        self.settle_before_branches(state, block, &mut out);\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "read_past_dropping_arm|read_before_short_circuit",
    ],
  },
  {
    name: "or-pattern-declared-order",
    breaks:
      "an or-pattern's variants are tested in the order the enum declares them, not the program's, `Uninitialized || Pending` of `Pending | Uninitialized` (ADR 0780)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                variants.sort_by_key(|v| written.iter().position(|(w, _)| w == v).unwrap_or(usize::MAX));\n",
    replace: "",
    tests: [
      "test/lowering-or-patterns.test.ts",
      "-t",
      "tested in the order written",
    ],
  },
  {
    name: "mir-assert-inverted",
    breaks: "a MIR `Assert` throws when its condition holds, not when it fails",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                    true => Expr::unary(js::UnaryOp::Not, cond),\n",
    replace: "                    true => cond,\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "drop_index_panic|drop_operand_leaves",
    ],
  },
  {
    name: "mir-folded-call-uncaught",
    breaks:
      "a call written where it's used, which may panic as something's there to drop, isn't in a `try`",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "            self.unwinding_to(state, unwind.filter(|_| ran), made, &mut out, Vec::new())?;\n",
    replace: "            self.unwinding_to(state, None, made, &mut out, Vec::new())?;\n",
    tests: ["test/corpus.test.ts", "-t", "drop|refcell_"],
  },
  {
    name: "mir-made-before-block-scoped",
    breaks:
      "what's made before a labeled block and read after it is declared inside it",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "        if let Some(last) = state.pending.iter().rposition(|(local, _)| !read.contains(local)) {\n",
    replace:
      "        if false && let Some(last) = state.pending.iter().rposition(|(local, _)| !read.contains(local)) {\n",
    tests: [
      "test/jsx-react-dev.test.ts",
      "-t",
      "react.dev's Diagram writes its images' srcs and caption test in place",
    ],
  },
  {
    name: "mir-return-unwritten",
    breaks:
      "a `return` the program wrote is taken for the end the function falls off, and goes",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                stmt.span = written;\n",
    replace: "                let _ = (stmt, written);\n",
    tests: [
      "test/lowering-early-exits.test.ts",
      "-t",
      "a unit function.s early returns joined to its end",
    ],
  },
  {
    name: "mir-bool-switch-negated-first",
    breaks:
      "`if c { a } else { b }` is `if (!c) { b } else { a }`, MIR's test of `false` first",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "            && targets.iter().all(|(value, _)| value == 0)\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^as_deref.rs$"],
    snapshots: true,
  },
  {
    name: "mir-switch-variant-order",
    breaks:
      "an enum's `match` tests its variants in their order, not its arms', `if (o == null)` first",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "            ordered.sort_by_key(|&(at, _)| at);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^as_deref.rs$"],
    snapshots: true,
  },
  {
    name: "mir-negated-if-flipped",
    breaks:
      "`if !c { a } else { b }`, which MIR tests as `c`, is `if (c) { b }`, its second branch first",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "            && !matches!((written_past_gotos(state.body, *target), written_past_gotos(state.body, yes)), (Some(no), Some(yes)) if no < yes)\n",
    replace: "",
    // Its own case, `if !active { .. }`, reads as written again by tidy; a
    // loop's `if !descriptor.enumerable` doesn't.
    tests: [
      "test/lowering-written-branches.test.ts",
      "-t",
      "a loop's body that ends in an if of a continue",
    ],
  },
  {
    name: "mir-written-return-past-drops",
    breaks:
      "a `break` the program wrote of a block it labeled, reached through the drops of what the block owns, isn't known as written, an `else` it didn't write after it",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "            && let [only] = preds[block].as_slice()\n",
    replace:
      "            && let [only] = preds[block].as_slice().get(..0).unwrap_or_default()\n",
    tests: ["test/lowering-labels.test.ts", "-t", "a break the program wrote through drops is its label's"],
  },
  {
    name: "mir-call-write-before-pending",
    breaks:
      'what a call writes, a setter\'s `n.textContent = "x"`, runs before a value made ahead of it that reads it, `<p>{n.textContent}</p>` reads the new one',
    file: "crates/rust-js-mir/src/structure.rs",
    find: "if called.iter().any(|s| matches!(s.kind, StmtKind::Assign(..))) && !state.pending.is_empty() {",
    replace:
      "if false && called.iter().any(|s| matches!(s.kind, StmtKind::Assign(..))) && !state.pending.is_empty() {",
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
    name: "mir-return-written-by-storage",
    breaks:
      "the value a `?` goes on to is returned at the `?`'s span, a `return` the program wrote, so its `?` stays nested, not a guard",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "            .filter(|s| !matches!(s.kind, StatementKind::StorageLive(_) | StatementKind::StorageDead(_)))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^store_get_mut.rs$"],
    snapshots: true,
  },
  {
    name: "mir-switch-cases-ungrouped",
    breaks:
      "an or-pattern's values that go to one block are a test each from MIR, the block copied into each, one copy's `const right` an assignment of nothing declared",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "            match grouped.iter_mut().find(|(_, t, _)| *t == target) {\n",
    replace:
      "            match None::<&mut (Expr, BasicBlock, Vec<u128>)> {\n",
    tests: [
      "test/lowering-matches.test.ts",
      "-t",
      "an or-pattern's values from MIR",
    ],
  },
  {
    name: "mir-if-span-of-condition",
    breaks: "MIR's `if` maps to its condition, not `if c`",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                span: self.js_span(arm.unwrap_or_else(|| self.if_span(span))),\n",
    replace:
      "                span: self.js_span(arm.unwrap_or(span)),\n",
    tests: ["test/emission.test.ts", "-t", "source map points from fib.js"],
  },
  {
    name: "mir-match-test-span-of-match",
    breaks:
      "MIR's test of a `match` arm maps to its `match`, not the arm's pattern",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "            let arm = written_at(state.body, target).and_then(|at| self.arm_span(span, at));\n",
    replace: "            let arm: Option<Span> = None;\n",
    tests: [
      "test/lowering-matches.test.ts",
      "-t",
      "a match's arms of more than one pattern are an if chain",
    ],
  },
  {
    name: "mir-switch-subject-made-each-test",
    breaks:
      "a `match` of a call's value calls it once per arm it tests, `match note(1, x) { 1 => .., 2 => .., _ => .. }`",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                    && made.has_effects()\n                {\n                    let value = std::mem::replace(made, Expr::taken());\n                    *made = self.spill(",
    replace:
      "                    && made.has_effects()\n                    && false\n                {\n                    let value = std::mem::replace(made, Expr::taken());\n                    *made = self.spill(",
    tests: ["test/corpus.test.ts", "-t", "^match_call_once.rs$"],
  },
  {
    name: "crate-future-awaited",
    breaks:
      "a call that asks a `Future` of a future of the crate's, `.await` of it, is made: `await [1]`",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                    return Err(self.unlowered_future_awaited(future, *fn_span));\n",
    replace: "                    let _ = future;\n",
    tests: ["test/diagnostics.test.ts", "-t", "an await of a crate's future"],
  },
  {
    name: "read-after-drop",
    breaks:
      "`d.len()` of a `D: Deref` returned is read after `d` is dropped, its destructor's change seen",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                    self.settle_reads(state, block, &mut out);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_guard_drop.rs$"],
  },
  {
    name: "captured-writer-gives-string",
    breaks:
      "a closure that writes its maker's `Formatter` gives back its maker's string",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                    (Some(_), _) if state.writes_captured => None,\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^closure_writes_captured_formatter.rs$",
    ],
    snapshots: true,
  },
  {
    name: "own-into-future-given-unrefused",
    breaks:
      "an `IntoFuture` of the crate's given to generic code is awaited there as a promise",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                    .and_then(|call| self.recognition().own_into_future_given(call))\n",
    replace:
      "                    .and_then(|call| self.recognition().own_into_future_given(call))\n                    .filter(|_| false)\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "an IntoFuture of the crate's given to generic code",
    ],
  },
  {
    name: "throwing-fn-returns-caught",
    breaks:
      'a throwing function\'s result is `return $try(() => "" + value);`, not the call, though its callers catch it',
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                            && self.krate.throwing.contains(&state.body.source.def_id()) =>\n",
    replace: "                            && false =>\n",
    tests: ["test/throwing-calls.test.ts", "-t", "is a throwing function"],
  },
  {
    name: "throwing-fn-returns-any-uncaught",
    breaks:
      "every function's result of a throwing call is the bare call, a `pub` one's too, whose JS callers read a `Result`",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                            && self.krate.throwing.contains(&state.body.source.def_id()) =>\n",
    replace: "                            && true =>\n",
    tests: ["test/throwing-calls.test.ts", "-t", "is a throwing function"],
  },
  {
    name: "defined-static-read-first",
    breaks:
      "`js::is_defined(js::symbol::to_string_tag)` reads `Symbol.toStringTag` before it tests, which throws where `Symbol` is undeclared",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                self.unread_defined_operands(state, func, args);\n",
    replace: "",
    tests: ["test/lowering-options.test.ts", "-t", "React's typeName"],
  },
  {
    name: "or-pattern-tests-each",
    breaks:
      'an or-pattern of an untagged enum\'s variants tests each, `Array.isArray(v) || (typeof v === "object" && !Array.isArray(v))`, not what they are together',
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                *test = self.mir_variant_test(\n                    (enum_value.clone(), held),\n                    *enum_ty,\n                    *adt,\n                    &variants,\n                    &spans,\n                    (*present, *none),\n                );\n",
    replace: "",
    tests: [
      "test/lowering-switches.test.ts",
      "-t",
      "a test of an object or an array is typeof object",
    ],
  },
  {
    name: "mir-labeled-block-end-unrecorded",
    breaks:
      "a block the program labeled, `'desc: { .. }`, is MIR's `bb41: { .. }` where JS keeps a label (ADR 0742)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                state.left.entry(to).or_insert(block);\n",
    replace: "",
    tests: [
      "test/lowering-labels.test.ts",
      "-t",
      "a block the program labeled keeps its label",
    ],
  },
  {
    name: "unwrapped-where-made-call-caught",
    breaks:
      "a throwing call whose `Result` is unwrapped where it's made is still `$try`'s, while its `unwrap()` is its value: the `Result` object, not the call's",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                    Value::Expr(e) if self.unwrapped_where_made(state, destination.local) => {\n                        Value::Expr(e.uncaught().unwrap_or(e))\n",
    replace:
      "                    Value::Expr(e) if self.unwrapped_where_made(state, destination.local) => {\n                        Value::Expr(e)\n",
    tests: [
      "test/throwing-calls.test.ts",
      "-t",
      "unwrap of a throwing call each branch",
    ],
  },
  {
    name: "written-break-unspanned",
    breaks:
      "a `break` the program wrote has no span, so tidy makes it an `else`, not where it's written (ADR 0790)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "            let written = left.map_or(js::Span::NONE, |(_, w)| w);",
    replace:
      "            let written = left.map_or(js::Span::NONE, |_| js::Span::NONE);",
    tests: [
      "test/lowering-labels.test.ts",
      "-t",
      "a block the program labeled that ends a case",
    ],
  },
  {
    name: "written-break-each-arm-of-its-value",
    breaks:
      "each arm of a `break 'desc match ..`'s value is taken for the `break`, so the `match` is split, `if (..) { desc = a; break; } desc = b; break;` (ADR 0790)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                .filter(|&found| !self.goes_on_to_break(state, to, found));",
    replace: "                .filter(|_| true);",
    tests: [
      "test/lowering-labels.test.ts",
      "-t",
      "a block the program labeled that ends a case",
    ],
  },
  {
    name: "written-break-not-through-merges",
    breaks:
      "a `break 'desc match ..` whose arms meet before it isn't the program's, so it's an `else` (ADR 0790)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "        if preds[block].len() < 2 || seen.contains(&block) {\n            return None;",
    replace:
      "        if preds[block].len() < 200 || seen.contains(&block) {\n            return None;",
    tests: [
      "test/lowering-labels.test.ts",
      "-t",
      "a block the program labeled that ends a case",
    ],
  },
  {
    name: "held-format-argument-unwound-bare",
    breaks:
      "a `format_args!` argument held out of the order it's made in, `const arg = first(..)`, is outside the `try` of what's live, so a panic in it drops nothing",
    file: "crates/rust-js-mir/src/structure.rs",
    find: " || self.acting_parts(state) < parts;\n",
    replace: ";\n",
    tests: ["test/corpus.test.ts", "-t", "^format_argument_unwinds.rs$"],
  },
  {
    name: "macro-fn-return-at-the-macro",
    breaks:
      "a `fn` in `js::export_default!`'s last `return` is placed at the macro, and the blank line before it is lost (ADR 0444, amended)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                        .filter(|block| !block.from_expansion())\n",
    replace: "                        .filter(|_| false)\n",
    tests: [
      "test/lowering-comments.test.ts",
      "-t",
      "a blank line before a function's last value in export_default! stays",
    ],
  },
  {
    name: "nullable-call-kept-undefined",
    breaks:
      "a variable of what a `#[rust_js::nullable]` function gave is returned `root ?? null`, as if it may be `undefined` (ADR 0620, amended)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                            Value::Expr(e) => Value::Expr(rust_js_lower::calls::given_by(self.tcx, func_ty, e)),\n",
    replace: "                            Value::Expr(e) => Value::Expr(e),\n",
    tests: [
      "test/lowering-nullable-reads.test.ts",
      "test/lowering-options.test.ts",
      "test/lowering-bool-ternaries.test.ts",
      "-t",
      "a variable of what a nullable function gave is returned and stored as it is|nullable function's call is the call|another module's function that gives a bool",
    ],
  },
  {
    name: "commented-break-unplaced",
    breaks:
      "a `break` with comments before it that MIR made the function's `return;` has no place, so the comments of a match arm's `break` are lost",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                Some(jump) if at.lo == done.lo && js::ends(jump) && jump.span.is_none() => jump.span = at,\n",
    replace: "",
    tests: ["test/lowering-comments-jumps.test.ts"],
  },
  {
    name: "commented-continue-unkept",
    breaks:
      "a `continue` with comments before it that tidy takes leaves them above the loop",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                _ if at.lo != done.lo => state.commented.jumps.push((done, at)),\n",
    replace: "",
    tests: ["test/lowering-comments-jumps.test.ts"],
  },
  {
    name: "held-keyed-read-boxed",
    breaks:
      "a keyed read whose local is held as its payload is `$someGet`'s box, which the local's reads take for the value itself: `Some(None)` read as `Some(Some(box))` (ADR 0884, amended)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                        Value::Expr(rust_js_lower::binding_calls::held_read(e))\n",
    replace: "                        Value::Expr(e)\n",
    tests: ["test/corpus.test.ts", "-t", "^map_none_held_as_null.rs$"],
  },
  {
    name: "option-bool-true-unswapped",
    breaks:
      "an `Option<bool>`'s `if let Some(true)` is `o !== false`, the negation of `Some(false)`'s test, not `o === true` (ADR 0298, the user's decision)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "} if self.option_bool_payload(tested.as_deref()) => Expr::bin(Op::Eq, *payload, Expr::bool(true)),\n",
    replace: "} if self.option_bool_payload(tested.as_deref()) => Expr::bin(Op::Ne, *payload, Expr::bool(false)),\n",
    tests: ["test/corpus.test.ts", "-t", "^option_bool_exact.rs$"],
    snapshots: true,
  },
  {
    name: "found-index-items-stepped",
    breaks:
      "the `iter()` a `position` searches is `$iter(children)`, which has no `indexOf`, a `TypeError`",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                    Value::Expr(e) if state.facts.found_index_items(block) => {\n",
    replace: "                    Value::Expr(e) if false && state.facts.found_index_items(block) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^position_index_of.rs$"],
  },
  {
    name: "guard-drop-reads-unsettled",
    breaks: "what's read through a guard whose drop writes nothing is read where it's used, past a call that writes its cell, `borrow.value.id` after `touch` emptied it (#452)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                } else if place.projection.is_empty() && self.is_guard(state.body.local_decls[place.local].ty) {\n",
    replace: "                } else if false && place.projection.is_empty() && self.is_guard(state.body.local_decls[place.local].ty) {\n",
    tests: ["test/corpus.test.ts", "-t", "guard_value_read_before_write"],
  },
  {
    name: "held-read-binding-only",
    breaks:
      "a std map's `get` writing a local held as its value is still `$someGet`'s box, which `=== undefined` and `=== null` read wrongly (rd547, ADR 1017 amended)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                                self.reads_held(def_id, args, destination.ty(&state.body.local_decls, self.tcx).ty)\n",
    replace: "                                bindings::reads_a_nullable_value(self.tcx, def_id, args)\n",
    tests: ["test/corpus.test.ts", "-t", "^untagged_null_option_held.rs$"],
  },
  {
    name: "owner-handle-undeclared",
    breaks:
      "an owner handle's cell is no `let`, so the call that makes the request assigns a variable never declared, which a module's strict JS throws at",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                out.push(StmtKind::Let(name, None).at(self.js_span(span)));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^owner_handle.rs$"],
  },
  {
    name: "constructed-get-coalesced",
    breaks: "a nullable field each constructor of its struct sets first, read for a nullable parameter, is given `?? null`: `prepare(response.moduleLoading ?? null)` (rd628)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                        let nulled = self.call_nulled(state, destination.local, (func, &args[..]));\n",
    replace: "                        let nulled = false;\n",
    tests: ["test/js-constructors.test.ts", "-t", "a nullable field each constructor sets first"],
  },
  {
    name: "box-outlives-own-variable",
    breaks:
      "a box of a variable of the body's own, gone once it unwinds, is copied back in a `finally` all the same, a `try` the JS needn't have (#487)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "    !matches!(&root.kind, js::ExprKind::Var(name)\n",
    replace: "    true || !matches!(&root.kind, js::ExprKind::Var(name)\n",
    tests: ["test/corpus.test.ts", "-t", "^written_through_before_a_panic.rs$"],
    snapshots: true,
  },
  {
    name: "box-unthrowing-wrapped",
    breaks:
      "a box given to a call that can't throw, of a variable a closure borrows, is copied back in a `finally` all the same, a `try` the JS needn't have (#487)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                            let throws = called.last().is_some_and(|s| !s.cannot_throw());\n",
    replace: "                            let throws = true;\n",
    tests: ["test/corpus.test.ts", "-t", "^written_through_before_a_panic.rs$"],
    snapshots: true,
  },
  {
    name: "untested-switch-subject-dropped",
    breaks:
      "a switch that goes one way alone, the rest unreachable, drops what it tests, `if loud(n) { unsafe { unreachable_unchecked() } }`'s call (rd643)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                    out.push(StmtKind::Expr(value).at(self.js_span(span)));\n",
    replace: "                    drop(value);\n",
    tests: ["test/corpus.test.ts", "-t", "^unreachable_hint.rs$"],
  },
  {
    name: "unreachable-hint-falls-through",
    breaks: "unreachable code a way doesn't leave out, a function's body of only `hint::unreachable_unchecked()`, throws nothing: what follows runs",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                out.push(self.unreachable_code(span));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^unreachable_hint.rs$"],
    snapshots: true,
  },
  {
    name: "nullable-call-ref-as-given",
    breaks:
      "a `&mut` a call gives to a nullable place, `&mut v[i]` of a nullable field's items, is its place alone, so a callee's box is copied back as it is, `fiber.lanes[0] = slot.value` (#541)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                                self.nullable_ref(state, pointee, target, &mut called)?\n",
    replace: "                                Value::Ref(target)\n",
    tests: ["test/lowering-nullable-borrows.test.ts", "-t", "written through a borrow"],
  },
  {
    name: "nullable-handle-place-unread",
    breaks:
      "the place a handle on a nullable place is on isn't read where a copy back is held on unwinding, so one from a callee that may throw is made in a `finally` (#541)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                                    let place = self.handled_place(place.clone());\n",
    replace: "                                    let place = place.clone();\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_written_through_borrows.rs$"],
    snapshots: true,
  },
  {
    name: "untold-refused-of-alike-arms",
    breaks:
      "a `match` whose arms do the same JS, no test left, is refused as though it told its classes by `instanceof` (#555)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "        if alike.is_none()\n            && let Value::Discriminant(_, enum_ty, ..) = &subject\n",
    replace: "        if let Value::Discriminant(_, enum_ty, ..) = &subject\n",
    tests: ["test/lowering-switches.test.ts", "-t", "arms are alike in JS"],
  },
  {
    name: "untold-of-untested-arms",
    breaks:
      "a `match` of an `Element | Blob` that tests its element, `nodeType === 1`, the blob what's left, is refused for the blob's class it never tests (#555)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                .filter(|(b, _)| tested_blocks.contains(b))\n",
    replace: "                .filter(|_| true)\n",
    tests: ["test/webapi-objects.test.ts", "-t", "told by their nodeType"],
  },
  {
    name: "untold-unrefused-in-match",
    breaks:
      "a `match` of `Dropped::Blob` and `Dropped::Element` is refused nowhere, its `instanceof Blob` `false` of another realm's (#555)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "            if let Some(refused) = self.untold_class(*enum_ty, &variants) {\n                return Err(rust_js_lower::refusal::error(self.tcx, span, refused));\n",
    replace: "            if let Some(refused) = self.untold_class(*enum_ty, &variants).filter(|_| false) {\n                return Err(rust_js_lower::refusal::error(self.tcx, span, refused));\n",
    tests: ["test/webapi-objects.test.ts", "-t", "told by their nodeType"],
  },
  {
    name: "assert-drops-unguarded",
    breaks: "a failed check's panic runs its drops unguarded, so a drop's panic there replaces it where Rust aborts (#847)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                let mut panic = rust_js_lower::drops::cleanup(unwound, None, js_span);\n",
    replace: "                let mut panic = unwound;\n",
    tests: ["test/corpus.test.ts", "-t", "^unwinding_drop_aborts_assert.rs$"],
  },
  {
    name: "pending-before-a-loop-dropped",
    breaks: "a call's argument made before a later `loop` argument is dropped as the loop begins, its name kept, a `ReferenceError` where the loop breaks (#1395)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "            // name nothing made (#1395).\n            let mut out = Vec::new();\n            self.flush(state, &mut out)?;\n",
    replace: "            // name nothing made (#1395).\n            let mut out = Vec::new();\n            self.flush(state, &mut Vec::new())?;\n",
    tests: ["test/corpus.test.ts", "-t", "^value_made_before_a_loop_argument.rs$"],
  },
  {
    name: "returned-calls-as-they-are",
    breaks: "what a function JS calls returns of a call, `id(value)` of JS's `null`, is `null` (#1840)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                        let value = self.one_none_returned(state, *destination, (None, callee), value, span)?;\n",
    replace: "",
    tests: given1840,
  },
  {
    name: "returned-iterator-of-a-closure",
    breaks: "a closure lowered inside a library's function giving back an `impl Iterator` has its own return converted too, `filter(|x| x % 2 == 1)`'s `bool` made a JS iterator, always truthy, so `[...Bag.odds(bag)]` is every item (#1999)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                        if state.body.source.def_id() == self.item\n                            && (self.krate.given.js_given)\n",
    replace: "                        if (self.krate.given.js_given)\n",
    tests: ["test/crates-iterators.test.ts"],
  },
  {
    name: "switch-split-never",
    breaks: "a switch's arms that meet at a body they share and another that goes on past it are a chain the other `break`s out of, a label MIR made, `bb16:` (#2144)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "        if inner.is_empty()\n            && let Some(joining) = self.split_of(state, block, merge)\n",
    replace: "        if false\n            && let Some(joining) = self.split_of(state, block, merge)\n",
    tests: ["test/lowering-or-patterns.test.ts", "-t", "arms that meet are one test"],
  },
  {
    name: "switch-split-of-the-crates-alone",
    breaks: "a switch over an enum JS may give, its tag held once, isn't split, so its arms keep the label MIR made (#2144, #2208)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                    && (!self.fields_read_once(ty) || self.tag_held(ty, *adt)))\n",
    replace: "                    && !self.fields_read_once(ty))\n",
    tests: ["test/lowering-or-patterns.test.ts", "-t", "arms that meet are one test"],
  },
  {
    name: "switch-split-of-assignments",
    breaks: "a switch whose arms meet at assignments alone is split too, nesting what tidy writes in each arm as one chain, `if (take && mapped.TAG === \"Single\")` (#2144)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "        if shared.iter().all(|&b| does_nothing(b)) {\n",
    replace: "        if false && shared.iter().all(|&b| does_nothing(b)) {\n",
    tests: ["test/lowering-or-patterns.test.ts", "-t", "meet at assignments alone"],
  },
  {
    name: "bounds-check-a-str",
    breaks: "an index out of bounds' payload, a `String`, downcasts as a `&str`",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "AssertKind::BoundsCheck { .. } => js::Payload::String,",
    replace: "AssertKind::BoundsCheck { .. } => js::Payload::Str,",
    tests: ["test/corpus.test.ts", "-t", "^panic_payload_kinds\\.rs$"],
  },
  {
    name: "drop-cleanup-unwritten",
    breaks: "a `Drop` whose drop runs something in JS (`unwinds`) is written without its cleanup, so a value's drop that panics leaves what its cleanup drops undropped (#2112's change D3)",
    file: "crates/rust-js-mir/src/structure.rs",
    find: "                        true => self.mir_unwind(state, *unwind)?,\n",
    replace: "                        true => Vec::new(),\n",
    tests: ["test/corpus.test.ts", "-t", "drop|refcell_"],
  },
];
