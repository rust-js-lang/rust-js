// Mutations of src/lower/mir/entries.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "swap-of-a-tuple-structs-field-a-handle",
    breaks: "`mem::swap(&mut spare.0, &mut pair.left)` gives `$exchange` the array and a handle, a `RangeError` (#1035)",
    file: "src/lower/mir/entries.rs",
    find: "                    if i < borrowed && fields && fixed_place(&e) && !matches!(e.kind, js::ExprKind::Var(_)) =>\n",
    replace: "                    if i < borrowed && fields && matches!(e.kind, js::ExprKind::Member(..)) =>\n",
    tests: ["test/corpus.test.ts", "-t", "^field_rewrite_singles.rs$"],
  },
  {
    name: "swap-into-a-nullable-place-as-given",
    breaks: "`mem::swap(&mut x, &mut other.0)` of a nullable parameter writes `undefined` for `None`, which its `=== null` test reads as `Some` (#1018)",
    file: "src/lower/mir/entries.rs",
    find: "            true => crate::lower::representation::NoneForm::Null.storage().written(e, false),\n",
    replace: "            true => e,\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_written_beside_a_wrapper.rs$"],
  },
  {
    name: "returned-write-result-unchecked",
    breaks: "a function that gives back a `fmt::Result` it holds, `let r = Err(fmt::Error); r`, returns the error as a value, where a call of it throws, so `{:?}` of it reads `Ok(())` (#1011)",
    file: "src/lower/mir/entries.rs",
    find: "            _ if crate::lower::display::may_be_write_error(&result) && self.krate.failing.any => {\n",
    replace: "            _ if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^fmt_error_as_a_value.rs$"],
  },
  {
    name: "write-all-value-its-length",
    breaks: "a `Vec`'s `write_all` as a value is `v.push(..)`'s length, an `io::Result<()>` read as an `Err` (#1010)",
    file: "src/lower/mir/entries.rs",
    find: "                if adds\n",
    replace: "                if false && adds\n",
    tests: ["test/corpus.test.ts", "-t", "^some_of_what_may_be_nullish.rs$"],
  },
  {
    name: "flattened-swap-destructured",
    breaks:
      "`mem::swap(&mut *state, &mut h.f)` of a flattened state's guard is `[task, h.f] = [h.f, ..]`, which rebinds the variable holding its class's object, not each key on it (#659)",
    file: "src/lower/mir/entries.rs",
    find: "        let destructured = !self.is_flattened_state(pointee);\n",
    replace: "        let destructured = true;\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_swapped_places.rs$"],
  },
  {
    name: "flattened-moved-out-as-it-is",
    breaks:
      "`mem::replace(&mut *state, ..)` of a flattened state's guard gives back its class's object, `const old = task;`, which the write then changes, not a copy of its variant's keys (#659)",
    file: "src/lower/mir/entries.rs",
    find: "        let moved_apart = self.is_unknown(pointee) || self.is_flattened_state(pointee);\n",
    replace: "        let moved_apart = self.is_unknown(pointee);\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_moved_out.rs$"],
  },
  {
    name: "mir-swap-one-way",
    breaks:
      "`mem::swap(&mut a, &mut b)` of two places writes `b` into `a`, and leaves `b`",
    file: "src/lower/mir/entries.rs",
    find: "                out.push(StmtKind::Assign(b, stored(b_stores, old)).at(js_span));\n",
    replace: "                let _ = (b, old);\n",
    tests: ["test/corpus.test.ts", "-t", "swap_replace"],
  },
  {
    name: "mir-entry-number-unhandled",
    breaks:
      "`*m.entry(k).or_insert(0) += 1` adds to the value read, writing nothing back",
    file: "src/lower/mir/entries.rs",
    find: "        if !self.is_cell(output) {\n",
    replace: "        if true || !self.is_cell(output) {\n",
    tests: ["test/corpus.test.ts", "-t", "entry_counts"],
  },
  {
    name: "mir-kept-entry-parts",
    breaks:
      "an entry kept in a variable is taken apart as `[m, k]`, not `$entry`'s `_0`",
    file: "src/lower/mir/entries.rs",
    find: "                let parts = Expr::member(entry, js::positional(0));\n",
    replace: "                let parts = entry;\n",
    tests: ["test/corpus.test.ts", "-t", "entry_kept"],
  },
  {
    name: "mir-take-variable-as-object",
    breaks:
      "`mem::take(&mut current)` of a variable is `$take(current, [])` from MIR, not `const old = current; current = [];`",
    file: "src/lower/mir/entries.rs",
    find: "Value::Expr(e) | Value::Place(e) if i < borrowed && variables && own(&e) => {",
    replace:
      "Value::Expr(e) | Value::Place(e) if false && i < borrowed && variables && own(&e) => {",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "taking a variable assigns the variable",
    ],
  },
  {
    name: "mir-failing-result-methods-refused",
    breaks: "MIR's `r.is_err()` of a `write!` that may fail is refused",
    file: "src/lower/mir/entries.rs",
    find: "        let may_fail = self.krate.failing.any && self.is_write_result(arg_tys[0].peel_refs());\n",
    replace: "        let may_fail = false;\n",
    tests: ["test/corpus.test.ts", "-t", "fmt_error_write"],
  },
  {
    name: "mir-failing-question-refused",
    breaks: "MIR's `write!(s, ..)?` that may fail is refused",
    file: "src/lower/mir/entries.rs",
    find: "        self.fmt_check(tried, self.js_span(span), out);\n        Ok(Value::Branch(Expr::undefined(), ty))\n",
    replace:
      '        let _ = tried;\n        Err(self.unsupported(span, "`?` of a write\'s result that may fail, from its MIR"))\n',
    tests: ["test/corpus.test.ts", "-t", "fmt_error_write"],
  },
  {
    name: "mir-fmt-result-arguments",
    breaks:
      "MIR's `{:?}` of a `fmt::Result` takes `Argument::new_debug` for a method of the result's",
    file: "src/lower/mir/entries.rs",
    find: "            && tcx\n                .impl_of_assoc(def_id)\n",
    replace:
      "            && true || tcx\n                .impl_of_assoc(def_id)\n",
    tests: ["test/corpus.test.ts", "-t", "fmt_error_write"],
  },
  {
    name: "or-insert-given-no-drop",
    breaks: "`or_insert` of a value with a destructor is given no drop",
    file: "src/lower/mir/entries.rs",
    find: "            (MapOp::OrInsert, Some(value)) => self.std_drop_function(value, span)?,\n",
    replace: "            (MapOp::OrInsert, Some(_)) => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^map_value_destructors.rs$"],
  },
  {
    name: "replaced-cell-refused",
    breaks:
      "`mem::replace` through a `&mut V` a store gives, which is a cell, is refused",
    file: "src/lower/mir/entries.rs",
    find: "                Value::Expr(e) if i < borrowed && self.is_cell(arg_tys[i]) => {\n",
    replace: "                Value::Expr(e) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^function_values_in_calls.rs$"],
  },
  {
    name: "io-residual-returned",
    breaks:
      "`?` of a value's `Result` into an `io::Result<()>` gives its `{ TAG }` object back, which its callers take for `Err`",
    file: "src/lower/mir/entries.rs",
    find: '            false => Expr::call_helper("$ioThrow", vec![crate::js::result::held(residual)]),\n',
    replace: "            false => residual,\n",
    tests: ["test/corpus.test.ts", "-t", "^io_unit_result.rs$"],
  },
  {
    name: "or-insert-key-dropped-twice",
    breaks:
      "`or_insert` of an entry made before drops the key `entry` dropped already",
    file: "src/lower/mir/entries.rs",
    find: "            Some(key) if fused => self.std_drop_function(key, span)?,\n",
    replace: "            Some(key) => self.std_drop_function(key, span)?,\n",
    tests: ["test/corpus.test.ts", "-t", "^map_entry_held_drops_its_key.rs$"],
  },
  {
    name: "or-insert-with-function-ungiven",
    breaks:
      "`or_insert_with` of a key that's there leaves its function, and what it holds, undropped",
    file: "src/lower/mir/entries.rs",
    find: "        list.extend(make_drop);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^unrun_function_drops.rs$"],
  },
  {
    name: "replaced-capture-not-in-place",
    breaks:
      "`mem::replace` of a captured field holding an iterator given as it is is refused, as if it weren't changed in place",
    file: "src/lower/mir/entries.rs",
    find: "                || !self.mut_ref_form(pointee).written_in_place()\n",
    replace: "                || !self.is_object(pointee)\n",
    tests: ["test/corpus.test.ts", "-t", "^replace_held_given_iterator.rs$"],
  },
  {
    name: "swap-some-fields-as-places",
    breaks:
      "a swap of a variable and a field takes the field for a place where the variable is an object, and swaps a handle's keys into the array",
    file: "src/lower/mir/entries.rs",
    find: "        let fields = (0..borrowed).all(|i| self.borrows_field(def_id, span, i));\n",
    replace:
      "        let fields = (0..borrowed).any(|i| self.borrows_field(def_id, span, i));\n",
    tests: ["test/corpus.test.ts", "-t", "^swap_replace.rs$"],
  },
  {
    name: "nullable-item-taken-undefined",
    breaks:
      "a nullable field's `Vec<Option<T>>` item taken, `v[i].take()`, is left `undefined`",
    file: "src/lower/mir/entries.rs",
    find: "            true => stored(a_stores, b),\n",
    replace: "            true => b,\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "Vec of Options holds null",
    ],
  },
  {
    name: "tried-place-read-twice",
    breaks:
      "`let p = s.next.borrow().clone()?;` reads `s.next` in its test and again in its binding, `if (s.next != null) { const p = s.next; .. }`, not once (ADR 0035, amended)",
    file: "src/lower/mir/entries.rs",
    find: "        if tried.reads_same() && !place {\n",
    replace: "        if tried.reads_same() {\n",
    tests: [
      "test/lowering-early-exits.test.ts",
      "-t",
      "a place a question mark tries is read once",
    ],
  },
  {
    name: "option-take-unmarked",
    breaks:
      "`root.pending.take()` then tested keeps its take first, `const pending = root.pending; root.pending = null; if (pending != null)`, not upstream's test of the place (ADR 0996)",
    file: "src/lower/mir/entries.rs",
    find: "        let b = match known == Std::OptionTake && made_here && js::taken_place(&a) {\n",
    replace:
      "        let b = match false && made_here && js::taken_place(&a) {\n",
    tests: ["test/lowering-takes.test.ts", "-t", "an Option taken then tested"],
  },
  {
    name: "swap-of-places-held",
    breaks:
      "`mem::swap(&mut a[0], &mut b[0])` is `const t = a[0]; a[0] = b[0]; b[0] = t;`, not JS's own `[a[0], b[0]] = [b[0], a[0]]` (#369)",
    file: "src/lower/mir/entries.rs",
    find: "        let place = |e: &Expr| !e.has_effects() && !matches!(e.kind, js::ExprKind::Var(_));\n",
    replace: "        let place = |_: &Expr| false;\n",
    tests: ["test/corpus.test.ts", "-t", "^swap_replace.rs$"],
    snapshots: true,
  },
  {
    name: "swap-of-variables-destructured",
    breaks:
      "`mem::swap(&mut a, &mut b)` of two variables is `[a, b] = [b, a]`, which reassigns the `const` a `let` that nothing else assigns is declared (#369)",
    file: "src/lower/mir/entries.rs",
    find: "        let place = |e: &Expr| !e.has_effects() && !matches!(e.kind, js::ExprKind::Var(_));\n",
    replace: "        let place = |e: &Expr| !e.has_effects();\n",
    tests: ["test/corpus.test.ts", "-t", "^swap_replace.rs$"],
  },
  {
    name: "generic-moved-out-shared",
    breaks:
      "generic code's `mem::replace(h.get(), v)` of a dictionary's `&mut Self::Out`, an in-place cell, gives the new value: what's moved out of a box not copied apart (#527)",
    file: "src/lower/mir/entries.rs",
    find: "            true => cx.moved_out(e, pointee),\n",
    replace: "            true => e,\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_cell_moved_out.rs$"],
  },
  {
    name: "replaced-holder-reached-twice",
    breaks:
      "`mem::replace(h.get(), v)` calls `h.get()` twice, once for its read and once for its write, printing what it prints twice",
    file: "src/lower/mir/entries.rs",
    find: "                    Value::Ref(match reached_by_a_call(&e) {\n",
    replace: "                    Value::Ref(match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_cell_moved_out.rs$"],
  },
  {
    name: "own-place-moved-out-apart",
    breaks:
      "`mem::take(&mut self.0)` of a generic struct's field copies the old value, `$apart(holder[0])`, where a write of the field rebinds it and nothing changes the old one (#527)",
    file: "src/lower/mir/entries.rs",
    find: "moved_apart && !cx.borrows_own_place(def_id, span, i)",
    replace: "moved_apart",
    tests: ["test/corpus.test.ts", "-t", "^drop_guard_of_a_closure.rs$"],
    snapshots: true,
  },
  {
    name: "checked-index-named-for-replace",
    breaks:
      "`mem::take(&mut links[0].to)` names the item, `const item$1 = $index(links, 0);`, where `$index` reaches the same place each time it's read",
    file: "src/lower/mir/entries.rs",
    find: "        js::ExprKind::Call(callee, args) if callee.helper_name().is_some_and(crate::runtime::names_a_place) => {\n            args.iter().any(reached_by_a_call)\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^weak_uncounted_uses.rs$"],
    snapshots: true,
  },
  {
    name: "clone-from-old-undropped",
    breaks:
      "`clone_from` doesn't drop what was there, as `*dest = source.clone()` does",
    file: "src/lower/mir/entries.rs",
    find: "            self.drop_value(old, ty, span, out)?;\n",
    replace: "            let _ = old;\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_from.rs$"],
  },
  {
    name: "clone-from-shared",
    breaks:
      "`clone_from` writes the source itself, not a clone, so a change to one is seen in the other",
    file: "src/lower/mir/entries.rs",
    find: "        let clone = self.clone_value(source, ty, span, out)?;\n        let given = vec![dest, Value::Expr(clone)];\n",
    replace:
      "        let given = vec![dest, Value::Expr(source)];\n",
    tests: ["test/corpus.test.ts", "-t", "^clone_from.rs$"],
  },
  {
    name: "entry-or-insert-unfused-for-nodes",
    breaks: "`or_insert` of a tree that may be std's nodes, or of a hash map of keys found by their own `Hash`, is `$orInsert` and `$mutGet`, which search again (#926, #1362)",
    file: "src/lower/mir/entries.rs",
    find: "        if once {\n            let entry = exprs.next().expect(\"the entry\");\n",
    replace: "        if false {\n            let entry = exprs.next().expect(\"the entry\");\n",
    tests: ["test/corpus.test.ts", "-t", "^btree_nodes_seen.rs$|^btree_nodes_generic.rs$|^map_entry_hashes_its_key_once.rs$"],
  },
];
