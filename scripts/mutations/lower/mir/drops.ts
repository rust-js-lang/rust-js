// Mutations of crates/rust-js-mir/src/drops.rs (ADR 0364).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-conditional-drop-static",
    breaks: "a value that may have moved is dropped as if it's always there",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "            (true, true, false) => Style::Conditional,\n",
    replace: "            (true, true, false) => Style::Static,\n",
    tests: ["test/corpus.test.ts", "-t", "drop|refcell_"],
  },
  {
    name: "mir-unwind-drops-nothing",
    breaks: "a panic drops nothing as it unwinds",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "                    self.mir_drop(state, block, place, true, span, &mut out)?;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "drop|refcell_"],
  },
  {
    name: "mir-unwind-never-caught",
    breaks:
      "what may panic is never in a `try`, so nothing is dropped as a panic unwinds",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "        let first = body.iter().take_while(|s| s.only_makes_values()).count();\n",
    replace: "        let first = body.len();\n",
    tests: ["test/corpus.test.ts", "-t", "drop|refcell_"],
  },
  {
    name: "mir-flags-never-set",
    breaks:
      "a drop flag keeps its first value, so what moved is dropped or what's there isn't",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "            if flagged[path] {\n",
    replace: "            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "drop|refcell_"],
  },
  {
    name: "mir-open-drop-other-variant",
    breaks:
      "an enum some of which has moved drops what's left of a variant when it's another",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "                    self.mir_variant_test(\n                        (subject.clone(), held.as_ref()),\n                        ty,\n                        adt,\n                        &[variant],\n                        &[],\n                        (false, none),\n                    )\n",
    replace:
      "                    Expr::unary(js::UnaryOp::Not, self.mir_variant_test(\n                        (subject.clone(), held.as_ref()),\n                        ty,\n                        adt,\n                        &[variant],\n                        &[],\n                        (false, none),\n                    ))\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "drop_partial|struct_update_drops|temporaries_taken_apart",
    ],
  },
  {
    name: "mir-drop-behind-pointer-skipped",
    breaks: "`self.inner = value` of a generic `T` drops the old value unrun",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "                    behind.insert(block);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "drop_generic"],
  },
  {
    name: "mir-cleanup-write-dropped",
    breaks: "a cleanup's write of the replacing value is left out",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "                            .and_then(|()| self.mir_assign(state, place, rvalue, span, &mut out))\n                    }\n",
    replace: "                            .and_then(|()| Ok(()))\n                    }\n",
    tests: ["test/corpus.test.ts", "-t", "drop_replace_panics"],
  },
  {
    name: "mir-drop-branch-unmapped",
    breaks:
      "MIR's drop of what `?` made, a part still there, reads `ControlFlow` as a place and is refused",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "            Some((tried, _)) => tried.clone(),\n            None => self.mir_place(state, place, span, out)?,\n",
    replace: "            _ => self.mir_place(state, place, span, out)?,\n",
    tests: ["test/corpus.test.ts", "-t", "question_drops"],
  },
  {
    name: "array-open-drop-refused",
    breaks:
      "an array some of whose items moved, `[a, b] = items`, is refused, not dropped item by item",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "                return self.open_drop_array(state, block, (place, path), size, cleanup, span, out);\n",
    replace:
      '                return Err(self.unsupported(span, &format!("an array of {size}")));\n',
    tests: ["test/corpus.test.ts", "-t", "^array_partial_moves.rs$"],
  },
  {
    name: "array-open-drop-first-first",
    breaks:
      "what's left of an array some of whose items moved drops first first, where rustc drops the last first",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "        for (part, subpath) in pieces.into_iter().rev() {\n",
    replace: "        for (part, subpath) in pieces.into_iter() {\n",
    tests: ["test/corpus.test.ts", "-t", "^array_partial_moves.rs$"],
  },
  {
    name: "array-open-drop-run-head",
    breaks: "a run of an array's items left is only its first item dropped",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "            let elem = match end - i {\n",
    replace: "            let elem = match 1 {\n",
    tests: ["test/corpus.test.ts", "-t", "^array_partial_moves.rs$"],
  },
  {
    name: "array-open-drop-run-unflagged",
    breaks:
      "an array's items left are dropped though the array moved whole, in another branch",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "                None => self.complete_drop(state, block, (part, path), span, &mut step)?,\n            }\n            steps.push(step);\n        }\n        self.each_ladder_step(steps, cleanup, span, out);\n        if !cleanup",
    replace:
      "                None => self.drop_place(state, part, span, &mut step)?,\n            }\n            steps.push(step);\n        }\n        self.each_ladder_step(steps, cleanup, span, out);\n        if !cleanup",
    tests: ["test/corpus.test.ts", "-t", "^array_partial_moves.rs$"],
  },
  {
    name: "unwinding-made-first-tried",
    breaks:
      "a closure made before a call that may panic is assigned in its `try`, `let key; try { key = (n) => n[0]; .. }`, not `const key = (n) => n[0];` before it",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "        let first = body.iter().take_while(|s| s.only_makes_values()).count();\n",
    replace:
      "        let first = if body.iter().all(Stmt::only_makes_values) { body.len() } else { 0 };\n",
    tests: ["test/corpus.test.ts", "-t", "^(cell_drops|take_destructors).rs$"],
    snapshots: true,
  },
  {
    name: "held-iterator-unheld",
    breaks:
      "an iterator its owner drops isn't held, so what it holds is dropped as it's done, before its scope's or statement's end (#206)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "        place.projection.is_empty()\n            && (self.dropped_by_closing(ty) || self.recognition().bounded_iterator(ty))\n",
    replace:
      "        false\n            && (self.dropped_by_closing(ty) || self.recognition().bounded_iterator(ty))\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^held_iterator_dropped_by_its_owner.rs$",
    ],
  },
  {
    name: "held-iterator-by-unwind-too",
    breaks:
      "an iterator dropped only as a panic unwinds past it, then moved into a chain, is held, so the chain's first side isn't dropped as it's done (#206)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "            !data.is_cleanup\n                && matches!(&data.terminator().kind, TerminatorKind::Drop {",
    replace:
      "            matches!(&data.terminator().kind, TerminatorKind::Drop {",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^adapter_source_drops.rs$"],
  },
  {
    name: "empty-cleanup-caught",
    breaks:
      "a call whose unwinding drops nothing, an `FnOnce`'s self given away, is `try { .. } catch (error) { throw error; }` (ADR 0873)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "        if cleanup.is_empty() {\n            out.extend(body);\n            return;\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_handed_to_js.rs$"],
    snapshots: true,
  },
  {
    name: "quiet-guard-released",
    breaks:
      "a momentary guard, checked and not counted, releases a borrow it never counted, `$unborrow(items)`, in a `try` of its own (ADR 0328, rd218); an unchecked guard's drop the elaboration leaves out (#700)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "        if place.projection.is_empty() && state.facts.quiet_cells().guards.contains(&place.local) {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_vec_bounds.rs$"],
    snapshots: true,
  },
  {
    name: "cleanup-flag-refused",
    breaks:
      "a drop flag optimized MIR's cleanup tests, another crate's, is refused, `this cleanup, from its MIR`, so a library's function that drops what it's given after asking its dispatcher can't be compiled by its consumer (ADR 0914)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "                    if let [(value, then)] = targets.iter().collect::<Vec<_>>()[..] =>\n",
    replace:
      "                    if false && let [(value, then)] = targets.iter().collect::<Vec<_>>()[..] =>\n",
    tests: ["test/crates.test.ts", "-t", "two crates: program_form"],
  },
  {
    name: "lent-iterator-unheld",
    breaks:
      "an iterator a variable steps by `next()` and then moves into `drop` isn't held, `$held(..)`, which its owner drops where Rust does (#392): its JS, as nothing shows the drop's timing apart now",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "        dropped || lent || self.moved_into_a_holder(body, local)\n",
    replace: "        dropped || self.moved_into_a_holder(body, local)\n",
    snapshots: true,
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(held_adapters_stepped_to_their_end|adapter_drops)\\.rs$",
    ],
  },
  {
    name: "generic-iterator-unheld",
    breaks:
      "a generic body's iterator of a type it's given, a parameter `impl Iterator` or `FromIterator`'s `items.into_iter()`, isn't held, `$held(..)`, which its owner drops where Rust does (#392): its JS, as nothing shows the drop's timing apart now",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "            && (self.dropped_by_closing(ty) || self.recognition().bounded_iterator(ty))\n",
    replace: "            && self.dropped_by_closing(ty)\n",
    snapshots: true,
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^held_adapters_stepped_to_their_end.rs$",
    ],
  },
  {
    name: "parameters-unheld",
    breaks:
      "a function's parameter it steps by `next()` and drops at its end isn't held, so what it's given drops at its first `None`, before `consume done` (#392)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: '                out.push(StmtKind::Expr(Expr::call_helper("$held", vec![name])).at(js::Span::NONE));\n',
    replace: "                let _ = name;\n",
    snapshots: true,
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^held_adapters_stepped_to_their_end.rs$",
    ],
  },
  {
    name: "compiled-std-parameters-held",
    breaks:
      "std's compiled struct, a `Skip` its method takes by value, is `$held(skip);` at the method's start, a call that holds nothing, as std's struct has no `hold()`",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "            if !rust_js_lower::recognition::is_compiled_std_iterator(self.tcx, ty)\n",
    replace: "            if true\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "constant-flags-never",
    breaks:
      "a library's optimized MIR's drop flag, a consumer's copy of `vec![t]`'s, is `const value = false;` and a `catch` testing it, `if (value) { dropT?.(t); }` (#381), and React's `runWithFiberInDEV`'s copy nests its flags' tests 2^k ways (#394)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "        .map(|(local, _)| local)\n        .collect();\n    flags.sort();",
    replace:
      "        .map(|(local, _)| local)\n        .filter(|_| false)\n        .collect();\n    flags.sort();",
    tests: ["test/crates.test.ts", "-t", "never crashes|settles its cleanup"],
  },
  {
    name: "constant-flags-of-any-use",
    breaks:
      "a `bool` set to other than a constant, or read other than by a test, is settled as a drop flag (#381)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "is_bool() && uses.0.get(local) == Some(n))",
    replace: "is_bool())",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(catch_unwind_drops|drop_update_panics).rs$",
    ],
    snapshots: true,
  },
  {
    name: "cleanup-switch-bool-only",
    breaks:
      "a cleanup that tests an enum's discriminant before it drops a variant's field, as a `core` built to unwind has, is refused (#424)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "                    if let [(value, then)] = targets.iter().collect::<Vec<_>>()[..] =>\n",
    replace:
      "                    if discr.ty(&state.body.local_decls, self.tcx).is_bool()\n                        && let [(value, then)] = targets.iter().collect::<Vec<_>>()[..] =>\n",
    tests: [
      "test/std-package.test.ts",
      "-t",
      "the committed @rust-js/std is std",
    ],
  },
  {
    name: "cleanup-discriminant-written",
    breaks:
      "a discriminant a cleanup's switch tests is written to a temporary the function never declares, a `ReferenceError` as a panic unwinds (#424)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "                        if matches!(&assign.1, mir::Rvalue::Discriminant(_)) && assign.0.as_local() == tested =>\n",
    replace:
      "                        if false && matches!(&assign.1, mir::Rvalue::Discriminant(_)) && assign.0.as_local() == tested =>\n",
    tests: [
      "test/std-package.test.ts",
      "-t",
      "the committed @rust-js/std is std",
    ],
  },
  {
    name: "cleanup-flag-tested-negated",
    breaks:
      "a cleanup's drop flag is tested negated, `if (!flag) { .. } else { drop(x); }`, which `settled_flags` leaves unsettled, a cleanup of every way it could be (#424)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "                        true => (self.value_expr(subject, span)?, targets.otherwise(), then),\n",
    replace:
      "                        true => (self.switch_test(&subject, ty, value, span, (None, None))?, then, targets.otherwise()),\n",
    tests: ["test/crates.test.ts", "-t", "settles its cleanup's drop flags"],
  },
  {
    name: "unchecked-guard-elaborated",
    breaks:
      "an unchecked guard's drop, of a cell no borrow of which is counted, is elaborated where its type drops, as another cell of it is counted: a flag says it, so a flattened state's `try_borrow` is declared first, `let _2;`, and its `Ok` kept as an object of the handle, `const value$1 = { TAG: \"Ok\", _0: state }` (#700)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "            if place.projection.is_empty() && quiet_guards.contains(&place.local) {\n",
    replace: "            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_tried.rs$"],
    snapshots: true,
  },
  {
    name: "cleanup-starts-at-its-chain",
    breaks: "a call's cleanup is its chain's first block, though it drops nothing and writes nothing, so calls that drop the same seem to unwind apart and what's made between them is held in a `const`: `vec![1, 2, 3, 4].into_iter().enumerate().rev()` beside a live `D` is refused (#723)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "                if drops.drops(block, place) {\n                    return Some(first.unwrap_or(block));\n",
    replace: "                if drops.drops(block, place) {\n                    let _ = first;\n                    return Some(start);\n",
    tests: ["test/corpus.test.ts", "-t", "^back_steps_beside_a_live_drop.rs$"],
  },
  {
    name: "box-contents-open-drop-as-a-struct",
    breaks: "a `Box` whose value moved out on one way, `take(*n)`, is dropped as a struct of its pointer and allocator, which drop nothing, so what it still holds is never dropped (#967)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "        if ty.is_box() {\n",
    replace: "        if false && ty.is_box() {\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_value_partly_moved_drops.rs$"],
  },
  {
    name: "box-contents-left-undropped",
    breaks: "what's left of a `Box`'s value some of which has moved, `Pair { a, ..*p }`'s `b`, is never dropped (#967)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "                Some(inner) => self.elaborate_drop(state, block, (interior, inner), cleanup, span, out)?,\n",
    replace: "                Some(_) => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_value_partly_moved_drops.rs$"],
  },
  {
    name: "moved-into-a-holder-unheld",
    breaks: "an iterator moved into a value that holds it, an `Rc<RefCell<..>>`, an `Option`, a field, an array or a tuple, isn't held, so it's dropped as it's stepped to its end, before what holds it is dropped (#1270)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "        dropped || lent || self.moved_into_a_holder(body, local)\n",
    replace: "        dropped || lent\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_held_in_a_place_dropped_with_it\\.rs$"],
  },
  {
    name: "boxed-iterator-unheld",
    breaks: "an iterator in a `Box`, `Box::new(items.into_iter())`, isn't held as the box is the iterator in JS, so it's dropped as it's stepped to its end, before the box is (#1270)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "        while let Some(inner) = ty.boxed_ty() {\n            ty = inner;\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^iterator_held_in_a_place_dropped_with_it\\.rs$"],
  },
  {
    name: "compiled-struct-in-a-holder-held",
    breaks: "std's compiled struct moved into a holder is wrapped `$held(..)`, which it has no `hold()` for, as it steps what it holds as Rust does (#243, #1270)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "        if rust_js_lower::recognition::is_compiled_std_iterator(self.tcx, body.local_decls[local].ty)\n            || rust_js_lower::recognition::is_std_crate(self.tcx, body.source.def_id())\n",
    replace: "        if rust_js_lower::recognition::is_std_crate(self.tcx, body.source.def_id())\n",
    tests: ["test/corpus.test.ts", "-t", "^copied_default_asks_its_drops_in_the_impls_terms\\.rs$"],
    snapshots: true,
  },
  {
    name: "adapter-of-it-a-holder",
    breaks: "generic code's iterator, what `IntoIterator::into_iter` of a parameter gives, whose item is `I::Item`, is taken for a holder, so it's held, `$held(IIntoIterator.intoIter(i))` (#1270)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "                && (!self.recognition().may_hold_by_bounds(made, iterator) || compiled(made, at))",
    replace: "                && (!self.implements_iterator(made) || compiled(made, at))",
    tests: ["test/corpus.test.ts", "-t", "^(adapter_drops|option_insert_given_drops)\\.rs$"],
    snapshots: true,
  },
  {
    name: "std-bodies-holder-held",
    breaks: "what std's own bodies move into theirs, a flatten's `frontiter`, is held, `$held(..)`, where std's compiled body drops it as it replaces it: the committed `@rust-js/std` isn't what the compiler compiles (#1270)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "        if rust_js_lower::recognition::is_compiled_std_iterator(self.tcx, body.local_decls[local].ty)\n            || rust_js_lower::recognition::is_std_crate(self.tcx, body.source.def_id())\n",
    replace: "        if rust_js_lower::recognition::is_compiled_std_iterator(self.tcx, body.local_decls[local].ty)\n",
    tests: ["test/std-package.test.ts"],
  },
  {
    name: "library-bodies-holder-unheld",
    breaks: "a library's body, which its consumer compiles from its optimized MIR, never holds what it moves into a value that holds it, so a `from_iter` that keeps what generic code's `collect()` gave it in an `Rc<RefCell<..>>` sees it dropped as it's stepped to its end, before it prints what it summed (#1366)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "            || rust_js_lower::recognition::is_std_crate(self.tcx, body.source.def_id())\n",
    replace: "            || !body.source.def_id().is_local()\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_iterator_held_in_a_place"],
  },
  {
    name: "kept-starts-unwritten",
    breaks: "a parameter React kept that a function changes in place itself is copied nowhere (#1723)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "        let mut out = self.kept_starts(state.body, &state.parts, &|local| state.locals.names[local].clone())?;\n",
    replace: "        let mut out = Vec::new();\n",
    tests: ["test/react-kept.test.ts", "-t", "copied where it's changed"],
  },
  {
    name: "cleanup-temporaries-one-each",
    breaks: "each statement of a cleanup must leave nothing it made, though what follows in it reads it: a 2018 closure's write through the `&mut` it captures, itertools' `coalesce`, refused (#1681)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "                        self.flag_sets(state, at, &mut out)\n                            .and_then(|()| self.mir_assign(state, place, rvalue, span, &mut out))\n",
    replace: "                        self.flag_sets(state, at, &mut out)\n                            .and_then(|()| self.mir_assign(state, place, rvalue, span, &mut out))\n                            .and_then(|()| match state.pending.is_empty() {\n                                true => Ok(()),\n                                false => Err(self.unsupported(span, \"this cleanup, from its MIR\")),\n                            })\n",
    tests: ["test/crates.test.ts", "-t", "coalesce_of_a_library_as_itertools_writes_it"],
  },
  {
    name: "compiled-struct-not-a-holder",
    breaks: "a value moved into std's compiled struct, generic code's `it.fuse()` of a `map` made in place by its caller, isn't held, so it closes itself at its end and drops the closure's capture before the `Fuse` drops it, where Rust's keeps it (#2039)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "                && (!self.recognition().may_hold_by_bounds(made, iterator) || compiled(made, at))",
    replace: "                && !self.recognition().may_hold_by_bounds(made, iterator)",
    tests: ["test/corpus.test.ts", "-t", "^fuse_holds_its_source_in_generic_code\\.rs$"],
  },
  {
    name: "moved-through-temporaries-unseen",
    breaks: "a value MIR moves into a temporary before a call, `fuse(move _4)` of `_4 = move _2`, isn't seen moved into what holds it, so it isn't held and closes itself at its end (#2039)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "matches!(operand, mir::Operand::Move(place) if values.contains(&place.local) && place.projection.is_empty());",
    replace: "matches!(operand, mir::Operand::Move(place) if place.local == local && place.projection.is_empty());",
    tests: ["test/corpus.test.ts", "-t", "^fuse_holds_its_source_in_generic_code\\.rs$"],
  },
  {
    name: "made-in-place-a-holder",
    breaks: "an adapter made in place of a compiled std type, a `rev()`'s `$rev`, is taken for std's struct, so what it's given is held, `$closing($held(SSource.items(source)), ..)`, though its own `hold()` holds it (#2039)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "                && !(self.frame.facts.in_place_adapters).made_at(self.frame.thir, at)\n",
    replace: "                && (!(self.frame.facts.in_place_adapters).made_at(self.frame.thir, at) || true)\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_iterator_dropped\\.rs$"],
    snapshots: true,
  },
  {
    name: "open-drop-otherwise-unflagged",
    breaks:
      "after a `match` one arm of which moves part of an enum and another all of it, `other => keep(other)`, the variants none takes apart are dropped whatever its flag says, so what the catch-all moved drops at once (#2064)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "                    Some(flag) => vec![StmtKind::If(Expr::var(flag), whole, None).at(self.js_span(span))],\n",
    replace: "                    Some(_) => whole,\n",
    tests: ["test/corpus.test.ts", "-t", "^match_binding_moves_what_a_partial_arm_keeps.rs$"],
  },
  {
    name: "ladder-steps-in-turn-unguarded",
    breaks: "what a partial move leaves is dropped one after another, so each field after one whose drop panics is never dropped, `drop b` lost after `a!` panics (#2146)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "        match cleanup {\n            true => out.extend(steps.into_iter().flatten()),\n",
    replace: "        match true {\n            true => out.extend(steps.into_iter().flatten()),\n",
    tests: ["test/corpus.test.ts", "-t", "^partial_move_rest_dropped_past_a_panic.rs$"],
  },
  {
    name: "drop-variants-none-unasked",
    breaks: "an `Option` of a value with a destructor, dropped by its variants, isn't told its one `None`, so its drop's test is loose (#2112 C S5)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "                    let none = self.place_none(state, place);\n",
    replace: "                    let none = None;\n",
    tests: ["test/corpus.test.ts", "-t", "^option_none_forms_through_captures.rs$"],
    snapshots: true,
  },
  {
    name: "rest-path-of-a-variant-its-own",
    breaks: "what's left of a variant some of whose fields moved, its fields with no path of their own, is dropped by the variant's own state, which says it's never there where another variant is, not by the enum's as rustc's `drop_subpath` drops it: alike as it runs, under the test of the discriminant, but its JS asks a flag of the variant's own where rustc's ladder asks the enum's or none (#2177)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "            (Some(ProjectionElem::Downcast(..)), Some(parent)) => parent,\n",
    replace: "            (Some(ProjectionElem::Downcast(..)), Some(_)) => parts,\n",
    tests: ["test/corpus.test.ts", "-t", "^(popped_variant_rest_dropped_at_the_match_end|or_pattern_part_moves).rs$"],
    snapshots: true,
  },
  {
    name: "drops-anything-by-its-own-path",
    breaks: "a `Drop` of a value each part of which moved, `?`'s `ControlFlow` once its value's taken or a tuple a pattern took apart, is said to run in JS by its own path's state, there still, so the cleanup only it unwinds to is judged, where rustc's elaboration makes the drop nothing and leaves no cleanup: unmatched (#2112's change D3)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "            LookupResult::Exact(path) => path,\n            LookupResult::Parent(_) => return elaboration.drops(block, place),\n        };\n        let mut runs = false;\n",
    replace: "            LookupResult::Exact(path) => return elaboration.drops(block, place) || path == path,\n            LookupResult::Parent(_) => return elaboration.drops(block, place),\n        };\n        let mut runs = false;\n",
    tests: ["test/drops-shadow.test.ts", "-t", "each cleanup verdict of the partial moves"],
  },
  {
    name: "drops-anything-of-a-quiet-drop",
    breaks: "a `Drop` of what JS drops nothing of is said to unwind in JS, so the cleanups only it reaches are judged as reached (#2112's change D3)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "        if self.drops(place.ty(&body.local_decls, self.tcx).ty) == Drops::Nothing {\n            return false;\n        }\n        let move_data = &elaboration.move_data;\n",
    replace: "        if self.drops(place.ty(&body.local_decls, self.tcx).ty) == Drops::Nothing {\n            return true;\n        }\n        let move_data = &elaboration.move_data;\n",
    tests: ["test/drops-shadow.test.ts", "-t", "each cleanup verdict of the partial moves"],
  },
  {
    name: "drops-anything-without-its-rest",
    breaks: "the rest of a part with parts of its own, an enum's variants not taken apart or a struct's fields with no path, isn't counted as what a `Drop` runs, so a drop of only that rest is said not to unwind and the writer leaves out its cleanup (#2112's change D3)",
    file: "crates/rust-js-mir/src/drops.rs",
    find: "                    elaboration.presence(block, elaboration.rest_path(node)).0\n                        && self.rest_holds_glue(body, move_data, node)\n",
    replace: "                    elaboration.presence(block, elaboration.rest_path(node)).0\n                        && self.rest_holds_glue(body, move_data, node)\n                        && false\n",
    tests: ["test/corpus.test.ts", "-t", "partial_move_rest_panics_before_what_was_made_first"],
  },
];
