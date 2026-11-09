// Mutations of src/lower/drops/parts.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "dropped-unwinding-not-rethrown",
    breaks: "what a call holds dropped as it unwinds swallows the panic, so `inspect`'s closure's panic ends no `catch_unwind` (#903)",
    file: "src/lower/drops/parts.rs",
    find: "    let mut handler = cleanup(dropped, js_span);\n    handler.push(StmtKind::Throw(Expr::var(\"error\")).at(js_span));\n    vec![StmtKind::TryCatch(body, Some(\"error\".into()), handler).at(js_span)]\n",
    replace: "    let handler = cleanup(dropped, js_span);\n    vec![StmtKind::TryCatch(body, Some(\"error\".into()), handler).at(js_span)]\n",
    tests: ["test/corpus.test.ts", "-t", "^inspect_unwinds_its_value.rs$"],
  },
  {
    name: "btree-drops-unsorted",
    breaks:
      "a `BTreeMap` JS gives drops its values in the order they went in (the crate's own are kept in order, ADR 0404, amended)",
    file: "src/lower/drops/parts.rs",
    find: "                        true => self.in_order_of(value, ty, span)?,\n",
    replace: "                        true => value,\n",
    tests: [
      "test/lowering-collections.test.ts",
      "-t",
      "a B-tree JS gives is dropped in its keys' order",
    ],
  },
  {
    name: "guard-drop-unrun",
    breaks: "a guard dropped leaves its cell borrowed",
    file: "src/lower/drops/parts.rs",
    find: '                out.push(StmtKind::Expr(Expr::call(Expr::helper("$unborrow"), vec![value])).at(js_span));\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "refcell_borrows"],
  },
  {
    name: "owned-iter-left-undropped",
    breaks:
      "what a `vec::IntoIter` hasn't given yet isn't dropped when a loop leaves early",
    file: "src/lower/drops/parts.rs",
    find: "            ty::Adt(..) if let Some(item) = self.recognition().owned_items(ty) => {\n",
    replace:
      "            ty::Adt(..) if let Some(item) = self.recognition().owned_items(ty).filter(|_| false) => {\n",
    tests: ["test/corpus.test.ts", "-t", "loop_drops"],
  },
  {
    name: "closure-problems-unfiltered",
    breaks:
      "a body lowered from its MIR is refused what THIR's facts refuse of more than its closures, moving part of a value with a destructor that MIR's drops handle",
    file: "src/lower/drops/parts.rs",
    find: "        for (span, what, _) in facts.problems.iter().filter(|(.., closure)| *closure) {\n",
    replace: "        for (span, what, _) in facts.problems.iter() {\n",
    tests: ["test/corpus.test.ts", "-t", "drop|partial|moved"],
  },
  {
    name: "dyn-cell-drop-of-value",
    breaks:
      "a `RefCell<dyn T>` dropped drops its `value` as the pair, not the cell it is",
    file: "src/lower/drops/parts.rs",
    find: "                let value = match self.cell_dyn_trait(inner).is_some() || self.is_value_cell(ty) {\n                    true => value,\n",
    replace:
      "                let value = match self.is_value_cell(ty) {\n                    true => value,\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_ref_cell_drop.rs$"],
  },
  {
    name: "closed-iterator-unclosed",
    breaks: "a `repeat`'s iterator dropped is walked as a struct, not closed",
    file: "src/lower/drops/parts.rs",
    find: "        recognition.closed_on_drop(ty).is_some()\n            || (recognition.held_function(ty))",
    replace: "        false\n            || (recognition.held_function(ty))",
    tests: ["test/corpus.test.ts", "-t", "^repeat_drops.rs$"],
  },
  {
    name: "receiver-drops-no-items",
    breaks: "a channel's receiver dropped leaves what's still queued undropped",
    file: "src/lower/drops/parts.rs",
    find: "                        values.extend(self.drop_function(args.type_at(0), span)?);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^channel_of_droppers.rs$"],
  },
  {
    name: "lock-value-undropped",
    breaks:
      "a `Mutex` or an `RwLock` holding a value with a destructor crashes the compiler, its fields asked of a cell",
    file: "src/lower/drops/parts.rs",
    find: "                if [StdItem::RefCell, StdItem::Mutex, StdItem::RwLock, StdItem::Cell]\n",
    replace: "                if [StdItem::RefCell, StdItem::Cell]\n",
    tests: ["test/corpus.test.ts", "-t", "^lock_destructors.rs$"],
  },
  {
    name: "map-keys-undropped",
    breaks:
      "dropping a map of keys and values with destructors drops its values alone",
    file: "src/lower/drops/parts.rs",
    find: "                            let mut value_dropped = Vec::new();\n                            self.drop_in(Expr::var(&key), key_ty, span, made, &mut body)?;\n",
    replace:
      "                            let mut value_dropped = Vec::new();\n",
    tests: ["test/corpus.test.ts", "-t", "^map_key_drops.rs$"],
  },
  {
    name: "returned-closure-drop-refused",
    breaks:
      "a closure made elsewhere that carries its drop is refused where it's dropped",
    file: "src/lower/drops/parts.rs",
    find: "            if self.frame.owner != def_id && self.krate.closure_facts.carries_its_drop(self.tcx, def_id))\n",
    replace:
      "            if false && self.krate.closure_facts.carries_its_drop(self.tcx, def_id))\n",
    tests: ["test/corpus.test.ts", "-t", "^returned_closure_drops.rs$"],
  },
  {
    name: "cell-held-undropped",
    breaks: "a `Cell` dropped drops none of its value",
    file: "src/lower/drops/parts.rs",
    find: "                if [StdItem::RefCell, StdItem::Mutex, StdItem::RwLock, StdItem::Cell]\n",
    replace:
      "                if [StdItem::RefCell, StdItem::Mutex, StdItem::RwLock]\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_drops.rs$"],
  },
  {
    name: "once-cell-held-undropped",
    breaks: "a `OnceCell` dropped drops none of its value",
    file: "src/lower/drops/parts.rs",
    find: "            ty::Adt(_, args) if self.is_std_type(ty, StdItem::OnceCell) => {\n",
    replace:
      "            ty::Adt(_, args) if false && self.is_std_type(ty, StdItem::OnceCell) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_drops.rs$"],
  },
  {
    name: "carried-dropped-by-places",
    breaks:
      "a closure that carries its drop is dropped by its maker's variables where it's made, once for each clone",
    file: "src/lower/drops/parts.rs",
    find: "            ty::Closure(..) if self.carries_its_drop(ty) => {\n                out.push(StmtKind::Expr(Expr::call_helper(\"$dispose\", vec![value])).at(js_span));\n",
    replace: "            ty::Closure(def_id, _) if self.carries_its_drop(ty) => {\n                self.drop_taken(*def_id, span, made, out)?;\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_clone_counted.rs$"],
  },
  {
    name: "fn-dyn-dropped-as-pair",
    breaks: "a `dyn Fn` is dropped by a dictionary it doesn't have",
    file: "src/lower/drops/parts.rs",
    find: "            ty::Dynamic(predicates, ..) if crate::lower::recognition::is_fn_dyn(self.tcx, predicates) => {\n                out.push(",
    replace:
      "            ty::Dynamic(predicates, ..) if false && crate::lower::recognition::is_fn_dyn(self.tcx, predicates) => {\n                out.push(",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_drops.rs$"],
  },
  {
    name: "value-cell-drops-value",
    breaks:
      "a `RefCell` of an object dropped drops its `value`, `undefined`, not the object, its items' destructors unrun",
    file: "src/lower/drops/parts.rs",
    find: "                let value = match self.cell_dyn_trait(inner).is_some() || self.is_value_cell(ty) {\n",
    replace:
      "                let value = match self.cell_dyn_trait(inner).is_some() {\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_counts.rs$"],
  },
  {
    name: "fn-once-disposes-itself",
    breaks:
      "a closure that carries its drop, called by `FnOnce`, drops itself in its own body by `$dispose` of a function that has no name there (ADR 0873)",
    file: "src/lower/drops/parts.rs",
    find: "            if self.frame.owner != def_id && self.krate.closure_facts.carries_its_drop(self.tcx, def_id))\n",
    replace:
      "            if self.krate.closure_facts.carries_its_drop(self.tcx, def_id))\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_once_drops_itself.rs$"],
  },
  {
    name: "std-struct-dropped-by-parts",
    breaks:
      "a `Peekable` std's compiled `fold` took is dropped again by its fields after, `drop e` twice (ADR 0913)",
    file: "src/lower/drops/parts.rs",
    find: "            || (crate::lower::recognition::is_compiled_std_iterator(self.tcx, self.reveal(ty))\n                && !crate::lower::recognition::is_held_compiled(self.tcx, self.reveal(ty)))",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^own_fold_through_adapters.rs$"],
  },
  {
    name: "drop-parts-unchained",
    breaks:
      "a closure's captures after one whose destructor panics are never dropped, its throw past them (#416, #846)",
    file: "src/lower/drops/parts.rs",
    find: "                dropped_after_unwinding(part.clone(), (unwinding, rest), js_span)\n",
    replace: "                part.iter().cloned().chain(rest).collect::<Vec<Stmt>>()\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_captures_dropped_past_a_panic.rs$"],
  },
  {
    name: "drop-items-unchained",
    breaks:
      "an array's or a `Vec`'s items after one whose destructor panics are never dropped, a `for…of` left by its throw (#416)",
    file: "src/lower/drops/parts.rs",
    find: "    if !may_throw(&body) {\n        return loop_dropped(items, pattern, body, js_span);\n",
    replace: "    if true {\n        return loop_dropped(items, pattern, body, js_span);\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_parts_past_a_panic.rs$"],
  },
  {
    name: "compiled-close-unchained",
    breaks:
      "a compiled std adapter's close stops at the first part whose drop panics: a peekable's peeked item after its source's, a chain's second side, never dropped (#677)",
    file: "src/lower/drops/parts.rs",
    find: "        Ok(each_dropped(each, js_span))\n",
    replace: "        let _ = js_span;\n        Ok(each.into_iter().flatten().collect())\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "projection-dropped-unnormalized",
    breaks: "an associated type its bounds say is another isn't dropped as that one, so `Iterator::count`'s closure inlined in std's `FlattenCompat::count` can't drop its item, and std's bodies don't compile (#736)",
    file: "src/lower/drops/parts.rs",
    find: "                    && normal != ty =>\n            {\n                self.drop_in(value, normal, span, made, out)?;\n",
    replace: "                    && normal != ty\n                    && false =>\n            {\n                self.drop_in(value, normal, span, made, out)?;\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "cleanup-unguarded",
    breaks: "what runs as a panic unwinds isn't `$cleanup`'s, so a drop's panic there replaces the panic where Rust aborts (#847)",
    file: "src/lower/drops/parts.rs",
    find: "    if !may_throw(&cleanup) {\n        return cleanup;\n",
    replace: "    if true {\n        return cleanup;\n",
    tests: ["test/corpus.test.ts", "-t", "^unwinding_drop_aborts_assert.rs$"],
  },
  {
    name: "dropped-after-finally-unguarded",
    breaks: "a drop that may panic after a body is its `finally`, which can't tell a panic as a panic unwinds, where Rust aborts (#847)",
    file: "src/lower/drops/parts.rs",
    find: "    if !may_throw(&dropped) {\n",
    replace: "    if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^xor_drop_panics.rs$"],
  },
  {
    name: "rc-drop-quiet-of-any",
    breaks: "an `Rc`'s drop is known never to throw though what its last drops may, so it's left unguarded as a panic unwinds (#847)",
    file: "src/lower/drops/parts.rs",
    find: "                    quiet = !may_throw(&body);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^unwinding_drop_aborts_assert.rs$"],
  },
  {
    name: "generic-drop-call-unmarked",
    breaks:
      "the call of a generic `Drop` impl that never unwinds as it's given its arguments isn't known to, so its guard drops after its `try` and in a `$cleanup` (#861)",
    file: "src/lower/drops/parts.rs",
    find: "        if self.drop_never_unwinds(d.did, args) {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_drops_judged_per_use.rs$"],
    snapshots: true,
  },
  {
    name: "drop-glue-rest-each-in-its-try",
    breaks:
      "a closure's captures' `catch` of one runs each later capture in a `try` of its own, so where one of those panics too the captures after it still drop before the abort, `drop 4`, where Rust aborts at once (#865)",
    file: "src/lower/drops/parts.rs",
    find: "                dropped_after_unwinding(part.clone(), (unwinding, rest), js_span)\n",
    replace: "                dropped_after_unwinding(part.clone(), (rest.clone(), rest), js_span)\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_captures_second_panic_aborts.rs$"],
  },
  {
    name: "close-of-quiet-items-may-throw",
    breaks:
      "closing an iterator of items whose drop can't panic, `it.return()`, is one that may throw, so its scope drops it after each `try` and in a `$cleanup` (#861)",
    file: "src/lower/drops/parts.rs",
    find: "                if !self.close_may_panic(ty, &mut IndexSet::default()) {\n",
    replace: "                if false && !self.close_may_panic(ty, &mut IndexSet::default()) {\n",
    tests: ["test/corpus.test.ts", "-t", "^(iterator_closes_of_quiet_items|peekable_drops).rs$"],
    snapshots: true,
  },
  {
    name: "close-item-unasked",
    breaks:
      "closing a `Peekable` of the crate's iterator isn't asked what it peeked at, of its `Item`, which its type doesn't name, so a close that drops a `Loud` that may panic is taken for one that never throws, in a `finally` (#861)",
    file: "src/lower/drops/parts.rs",
    find: "                .is_some_and(|item| self.drop_may_panic(item))\n",
    replace: "                .is_some_and(|item| self.drop_may_panic(item) && false)\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_closes_of_quiet_items.rs$"],
    snapshots: true,
  },
  {
    name: "close-adapters-unwalked",
    breaks:
      "closing a `map()` of a `Peekable` isn't asked what the inner one peeked at, so a close that drops a `Loud` that may panic is taken for one that never throws (#861)",
    file: "src/lower/drops/parts.rs",
    find: "                && args.types().any(|arg| self.close_may_panic(arg, seen)))\n",
    replace: "                && args.types().any(|arg| self.close_may_panic(arg, seen) && false))\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_closes_of_quiet_items.rs$"],
    snapshots: true,
  },
  {
    name: "hash-map-drop-goes-on",
    breaks: "a hashed map's drop of its keys or values alone goes on past one whose drop panics, where hashbrown's stops (#920)",
    file: "src/lower/drops/parts.rs",
    find: "                    out.push(loop_dropped(items, js::Pattern::Name(name), body, js_span));\n",
    replace: "                    out.push(items_dropped(items, js::Pattern::Name(name), body, js_span));\n",
    tests: ["test/corpus.test.ts", "-t", "^hash_map_drop_stops_at_a_panic.rs$"],
  },
  {
    name: "hash-map-entries-drop-goes-on",
    breaks: "a hashed map's drop of its keys and values goes on past an entry whose drop panics, where hashbrown's stops (#920)",
    file: "src/lower/drops/parts.rs",
    find: "                        false => loop_dropped(entries, pattern, body, js_span),\n",
    replace: "                        false => items_dropped(entries, pattern, body, js_span),\n",
    tests: ["test/corpus.test.ts", "-t", "^hash_map_drop_stops_at_a_panic.rs$"],
  },
  {
    name: "drop-glue-cleanup-unguarded",
    breaks:
      "a drop after a body leaves its cleanup unguarded where it's made, so a closure's drop of its captures, an arrow no tidy guards, goes on past a second panic where Rust aborts (#938)",
    file: "src/lower/drops/parts.rs",
    find: "    after(body, (cleanup(unwinding, js_span), dropped), js_span)\n",
    replace: "    after(body, (unwinding, dropped), js_span)\n",
    tests: ["test/corpus.test.ts", "-t", "^closure_captures_dropped_after_a_sum_abort.rs$"],
  },
];
