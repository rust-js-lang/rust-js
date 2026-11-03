// Mutations of src/lower/mir/nested_options.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "held-some-null-for-none",
    breaks:
      "a `Some` of a value `null` but never `undefined`, held as its value, is written as an inner `Option`'s, `fallback ?? null` (rd547, ADR 1017 amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "            Some(operand) if form != Held::Nested => self.mir_expr(state, operand, out)?,\n",
    replace: "            Some(operand) if form == Held::Value => self.mir_expr(state, operand, out)?,\n",
    tests: ["test/corpus.test.ts", "-t", "^untagged_null_option_held.rs$"],
    snapshots: true,
  },
  {
    name: "nested-option-none-null",
    breaks:
      "a held `Option` of an `Option`'s `None` is `null`, its `Some(None)`'s, so the two are equal (ADR 1017)",
    file: "src/lower/mir/nested_options.rs",
    find: "            None => Expr::undefined(),\n",
    replace: "            None => Expr::null(),\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_held_as_payload.rs$"],
  },
  {
    name: "nested-option-some-none-undefined",
    breaks:
      "a held `Option` of an `Option` made of an `Option` whose `None` is `undefined` keeps it, so its `Some(None)` is its `None` (ADR 1017)",
    file: "src/lower/mir/nested_options.rs",
    find: "                js::Storage::NullForNone.written(held, nulled)\n",
    replace: "                { let _ = nulled; held }\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_held_as_payload.rs$"],
  },
  {
    name: "nested-field-unseen",
    breaks: "a dictionary's nullable `Option<Option<T>>` field is read as ADR 0051's box: `{ value: null }` is `None` (#470)",
    file: "src/lower/mir/nested_options.rs",
    find: "            ty::Adt(adt, _) if adt.is_struct() => bindings::is_held(self.tcx, &adt.non_enum_variant().fields[field]),\n",
    replace: "            ty::Adt(adt, _) if adt.is_struct() => bindings::holds_value(self.tcx, &adt.non_enum_variant().fields[field]),\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "a dictionary's nullable Option of an Option"],
  },
  {
    name: "nested-field-boxed-as-held",
    breaks: "a nested field read whole is given as it's held to code that boxes: its `Some(None)`, `null`, is shown `None` (#470)",
    file: "src/lower/mir/nested_options.rs",
    find: "        let some = self.some(held.clone());\n        Expr::cond(Expr::bin(Op::Eq, held, self.held_none(ty)), Expr::undefined(), some)\n",
    replace: "        held\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_nullable_field.rs$"],
  },
  {
    name: "nested-field-given-none-undefined",
    breaks: "a boxed `Some(None)` given a nested field is `undefined`, read back as `None` (#470)",
    file: "src/lower/mir/nested_options.rs",
    find: "            false => js::Storage::NullForNone.written(value, false),\n        };\n        Expr::cond(",
    replace: "            false => value,\n        };\n        Expr::cond(",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_nullable_field.rs$"],
  },
  {
    name: "nested-field-given-boxed",
    breaks: "what a struct literal gives a nested field is ADR 0051's box, `{ $someNone: 0 }`, where JS reads `null` (#470)",
    file: "src/lower/mir/nested_options.rs",
    find: "            true => self.held_of_boxed(value, operand.ty(state.body, self.tcx), out),\n            false => value,\n",
    replace: "            true => value,\n            false => value,\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "a dictionary's nullable Option of an Option"],
  },
  {
    name: "nested-field-refs-unheld",
    breaks: "`entry.value.is_some()` reads the field's box, `$some(..)`, not `entry.value !== undefined` (#470)",
    file: "src/lower/mir/nested_options.rs",
    find: "                    if writes[to] == 1 && self.nested_field(body, of.as_ref()))\n",
    replace: "                    if writes[to] == 1 && self.nested_field(body, of.as_ref()) && false)\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "a dictionary's nullable Option of an Option"],
  },
  {
    name: "nested-field-copies-boxed",
    breaks: "a copy of a nested field, a destructured parameter's `value`, is boxed, `value.flatten()` `$someValue(value)` (#470)",
    file: "src/lower/mir/nested_options.rs",
    find: "        let of_field = |local: Local| of_fields.contains(&local);\n",
    replace: "        let of_field = |local: Local| of_fields.contains(&local) && false;\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "a dictionary's nullable Option of an Option"],
  },
  {
    name: "nested-option-mentions-unchecked",
    breaks:
      "an `Option` of an `Option` given to a function is held as its payload too, so the function, which boxes, reads its `Some(None)`, `null`, as `None` (ADR 1017)",
    file: "src/lower/mir/nested_options.rs",
    find: "                    && !allowed.contains(&(at, local))\n",
    replace: "                    && false\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_held_as_payload.rs$"],
  },
  {
    name: "nested-option-copies-unlinked",
    breaks:
      "an `Option` of an `Option` copied to one given to a function is held as its payload while the copy is boxed (ADR 1017)",
    file: "src/lower/mir/nested_options.rs",
    find: "                if out.contains(&x) || out.contains(&y) {\n",
    replace: "                if out.contains(&x) && out.contains(&y) {\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_held_as_payload.rs$"],
  },
  {
    name: "nested-inner-given-on-unboxed",
    breaks:
      "the inner `Option` of one held as its payload, given on, is its `null`: `first(undefined, f)` calls `f(null)` (rd489, ADR 1017 amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "                            && (!of.projection[2..].iter().all(|elem| *elem == PlaceElem::Deref)\n",
    replace: "                            && (true\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "gives on is undefined"],
  },
  {
    name: "nested-inner-through-reference-unchecked",
    breaks:
      "the inner `Option` an `Option<&Option<T>>` held as its payload refers to, given on, is its `null` (rd489, ADR 1017 amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "                            && (!of.projection[2..].iter().all(|elem| *elem == PlaceElem::Deref)\n",
    replace: "                            && (!of.projection[2..].is_empty()\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "gives on is undefined"],
  },
  {
    name: "nested-inner-copied-on-unchecked",
    breaks:
      "the inner `Option` of one held as its payload, copied out and returned, is its `null` (rd489, ADR 1017 amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "                                | Rvalue::CopyForDeref(of) => of.local == held && its_payload(of),\n",
    replace: "                                | Rvalue::CopyForDeref(of) => of.local == held,\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "gives on is undefined"],
  },
  {
    name: "nested-inner-test-boxed",
    breaks:
      "an `Option` of an `Option` whose inner `Option` is only tested, `inner.is_some()`, keeps its box, `$some(value)`, not `last != null` (ADR 1017 amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "        writes[held] == 1\n            && (mentions.iter())\n",
    replace: "        false\n            && (mentions.iter())\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "gives on is undefined"],
  },
  {
    name: "nested-inner-test-reference-unseen",
    breaks:
      "an `Option` of an `Option` whose inner `Option` is given by a reference to `is_some()` keeps its box (ADR 1017 amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "                tested.insert(reference);\n",
    replace: "",
    tests: ["test/lowering-nested-options.test.ts", "-t", "gives on is undefined"],
  },
  {
    name: "nested-option-is-some-boxed",
    breaks:
      "`last.is_some()` of an `Option` of an `Option` held where it's made keeps its box, `$some(item)`",
    file: "src/lower/mir/nested_options.rs",
    find: "                        through(&a.node)\n",
    replace: "                        None\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_tests.rs$"],
    snapshots: true,
  },
  {
    name: "nested-option-some-none-eq-boxed",
    breaks:
      "`last == Some(None)` of an `Option` of an `Option` held where it's made keeps its box, `$eq(last, { $someNone: 0 })`",
    file: "src/lower/mir/nested_options.rs",
    find: "                            (Some(a), None) if self.nested_some_none(body, &b.node) => Some(a),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_tests.rs$"],
    snapshots: true,
  },
  {
    name: "nested-option-keyed-read-out",
    breaks:
      "a keyed read of a value a collection holds `None` of as `null` doesn't write a local held as its payload, so it's `$someGet`'s box, `$eq(.., { $someNone: 0 })` (ADR 0884, amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "                && self.reads_held(def_id, args, decls[to].ty)\n            {\n                allowed.insert((body.terminator_loc(block), to));",
    replace:
      "                && self.reads_held(def_id, args, decls[to].ty)\n                && false\n            {\n                allowed.insert((body.terminator_loc(block), to));",
    tests: ["test/js-collections.test.ts", "-t", "a Map's value is any value"],
  },
  {
    name: "held-whole-payload-only-tested",
    breaks:
      "an `Option` of a value `null` but never `undefined` read whole, `Some(entry) => describe(entry)`, keeps its box, `$someValue(get)`, as an inner `Option`'s read whole does (rd547, ADR 1017 amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "                                || self.defined_payload(decls[of.local].ty).is_some()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^untagged_null_option_held.rs$"],
    snapshots: true,
  },
  {
    name: "held-cloned-unseen",
    breaks:
      "`cache.get(id).cloned()` of a value `null` but never `undefined` keeps its box, `$someGet(cache, id)` (rd547, ADR 1017 amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "                && self.recognition().classify(def_id, generic_args) == Some(Std::OptionCloned)\n",
    replace: "                && self.recognition().classify(def_id, generic_args) == Some(Std::OptionCloned)\n                && false\n",
    tests: ["test/corpus.test.ts", "-t", "^untagged_null_option_held.rs$"],
    snapshots: true,
  },
  {
    name: "held-cloned-unlinked",
    breaks:
      "a `cloned()` given to a function is boxed while what it clones is held as its value, so the function reads its `Loaded`, `null`, as `None` (rd547, ADR 1017 amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "                links.push((to, from));\n            }\n            // An `==` of two of them",
    replace: "            }\n            // An `==` of two of them",
    tests: ["test/corpus.test.ts", "-t", "^untagged_null_option_held.rs$"],
  },
  {
    name: "held-cloned-copies",
    breaks:
      "a `cloned()` of an `Option` of a value its clone copies, a `Vec`'s, is held as its value too (rd547, ADR 1017 amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "                && !self.needs_clone(decls[to].ty)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^untagged_null_option_held.rs$"],
    snapshots: true,
  },
  {
    name: "held-map-get-any-value",
    breaks:
      "a std map's `get` of an `Option`, whose `None` the map holds as `undefined`, is `values.get(at)`, its `Some(None)` read as `None` (rd547, ADR 1017 amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "            || (self.defined_payload(ty).is_some()\n",
    replace: "            || (true\n",
    tests: ["test/corpus.test.ts", "-t", "^untagged_null_option_held.rs$"],
  },
  {
    name: "held-eq-of-any",
    breaks:
      "an `==` of two `Option`s of a value `null` but never `undefined` is read as an `Option` of an `Option`'s, which it isn't: a panic (rd547, ADR 1017 amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "                && self.nested_payload(decls[x].ty).is_some()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^untagged_null_option_held.rs$"],
  },
  {
    name: "with-borrow-result-unseen",
    breaks:
      "a thread-local's `with_borrow(|cache| cache.get(id).cloned())` writes its local boxed, `$someValue(entry)`, where its closure's result is held as the value (rd547, ADR 1017 amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "                && self.result_held(closure)\n",
    replace: "                && false\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "with_borrow gives back"],
  },
  {
    name: "with-borrow-result-of-any-closure",
    breaks:
      "a thread-local's `with_borrow` of a closure whose body boxes its result, a call's, writes a local held as the value, which that closure can't give: refused (rd547, ADR 1017 amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "                && self.result_held(closure)\n",
    replace: "                && true\n",
    tests: ["test/corpus.test.ts", "-t", "^untagged_null_option_held.rs$"],
  },
  {
    name: "held-result-not-given-back",
    breaks:
      "a closure's result held as its payload isn't given back where it returns, so it's boxed, and so is the local its `with_borrow` writes (rd547, ADR 1017 amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "            if result && matches!(data.terminator().kind, mir::TerminatorKind::Return) {\n",
    replace: "            if false && matches!(data.terminator().kind, mir::TerminatorKind::Return) {\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "with_borrow gives back"],
  },
  {
    name: "held-result-not-a-candidate",
    breaks:
      "a closure's result isn't among the locals held as their payload, so it's boxed, and so is the local its `with_borrow` writes (rd547, ADR 1017 amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "            .filter(|&local| local.as_usize() > body.arg_count || (result && local == mir::RETURN_PLACE))\n",
    replace: "            .filter(|&local| local.as_usize() > body.arg_count)\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "with_borrow gives back"],
  },
  {
    name: "held-results-of-any-local",
    breaks:
      "a closure whose result its maker gives on, boxed, gives it back held as the value: `owned` reads the `null` of `Loaded` as `None` (rd547, ADR 1017 amended)",
    file: "src/lower/mir/nested_options.rs",
    find: "            .filter(|(to, _)| nested.contains(to))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^untagged_null_option_held.rs$"],
  },
  {
    name: "held-capture-unseen",
    breaks:
      "a closure's capture of a nested field is read as ADR 0051's box in its body, `stringify($someValue(captured))` of a `$some` made where it's captured, where the field is read as it's held, `stringify(entry.value)` (rd631)",
    file: "src/lower/mir/nested_options.rs",
    find: "        let captured = self.tcx.closure_captures(local).get(capture.as_usize())?;\n",
    replace: "        let captured = self.tcx.closure_captures(local).get(capture.as_usize()).filter(|_| false)?;\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_field_captured.rs$"],
    snapshots: true,
  },
  {
    name: "held-capture-given-boxed",
    breaks:
      "a closure's capture of a nested field its maker reads as the box is given it boxed, which its body reads as it's held: `once`'s `Some(None)` name is `$some(null)`, shown `[object Object]` (rd631)",
    file: "src/lower/mir/nested_options.rs",
    find: "                let boxed = self.value_expr(value, Span::default())?;\n                Ok(Value::Expr(self.held_of_boxed(boxed, ty.expect(\"a capture\"), out)))\n",
    replace: "                Ok(value)\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_field_captured.rs$"],
  },
  {
    name: "held-capture-by-value-unseen",
    breaks:
      "a closure's capture of a nested field it took by value is read as the box in its body, which its maker gives held: `rewritten`'s `Some(None)` is read `None` (rd631)",
    file: "src/lower/mir/nested_options.rs",
    find: "            Some(false) => read.is_empty(),\n",
    replace: "            Some(false) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_field_captured.rs$"],
  },
  {
    name: "held-capture-mention-refused",
    breaks:
      "a borrow of a nested field a closure captures is boxed where it's made, and held again for the closure, `entry.value === undefined ? undefined : $someValue(..) ?? null`, where it's the field (rd631)",
    file: "src/lower/mir/nested_options.rs",
    find: "                                && (if by_ref {\n                                    field_refs.contains(&from)\n",
    replace: "                                && (if by_ref {\n                                    false\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_field_captured.rs$"],
    snapshots: true,
  },
  {
    name: "held-capture-borrow-spilled",
    breaks:
      "a borrow of a nested field a closure captures is copied to a `const captured = entry.value;` where it's made, where the closure reads the field (rd631)",
    file: "src/lower/mir/nested_options.rs",
    find: "                        Value::Expr(place) if by_ref && place.reads_same() => Value::Place(place),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_field_captured.rs$"],
    snapshots: true,
  },
  {
    name: "held-value-none-undefined",
    breaks:
      "a `None` made of a nullable field of what may be `undefined` is `undefined`, its `Some` (rd645)",
    file: "src/lower/mir/nested_options.rs",
    find: "            None if form == Held::Value => Expr::null(),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^nullable_field_of_undefined.rs$"],
  },
  {
    name: "held-value-none-tested-undefined",
    breaks:
      "a nullable field of what may be `undefined` is held and tested with `undefined` its `None` (rd645)",
    file: "src/lower/mir/nested_options.rs",
    find: "            Held::Value => Expr::null(),\n",
    replace: "            Held::Value => Expr::undefined(),\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_field_of_undefined.rs$"],
  },
  {
    name: "held-value-unboxed-null",
    breaks:
      "a box stored in a nullable field of what may be `undefined` is `$someValue(x) ?? null`, its `Some(undefined)` `null`, a `None` (rd645)",
    file: "src/lower/mir/nested_options.rs",
    find: "            true => value,\n            false => js::Storage::NullForNone.written(value, false),\n",
    replace: "            true => js::Storage::NullForNone.written(value.clone(), false),\n            false => js::Storage::NullForNone.written(value, false),\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_field_of_undefined.rs$"],
  },
  {
    name: "held-any-of-unheld",
    breaks:
      "`js::any_of` of an `Option` of what may be `undefined` gives the box a local holds, `$some(undefined)`, not what a nullable place holds (rd645)",
    file: "src/lower/mir/nested_options.rs",
    find: "        if !crate::lower::recognition::is_any_of(self.tcx, def_id) || self.undefined_payload(ty).is_none() {\n",
    replace: "        if true {\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "what may be undefined"],
  },
  {
    name: "any-of-of-a-held-local-boxed",
    breaks:
      "a local of a nullable field's read given to `js::any_of` is boxed, `x === null ? undefined : $some(x)` (rd645)",
    file: "src/lower/mir/nested_options.rs",
    find: "                && crate::lower::recognition::is_any_of(self.tcx, def_id)\n                && let [given] = &args[..]\n",
    replace: "                && crate::lower::recognition::is_any_of(self.tcx, def_id)\n                && false\n                && let [given] = &args[..]\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "what may be undefined"],
  },
  {
    name: "undefined-payload-local-unheld",
    breaks:
      "a local of a nullable field's read of what may be `undefined` is boxed where it's read, though it's only tested (rd645)",
    file: "src/lower/mir/nested_options.rs",
    find: "                    || self.defined_payload(ty).is_some()\n                    || self.undefined_payload(ty).is_some())\n",
    replace: "                    || self.defined_payload(ty).is_some())\n",
    tests: ["test/lowering-nested-options.test.ts", "-t", "what may be undefined"],
  },
  {
    name: "undefined-payload-payload-unread",
    breaks:
      "what a local's `Some` of what may be `undefined` holds isn't read where it's held, so the local is boxed (rd645)",
    file: "src/lower/mir/nested_options.rs",
    find: "                                || self.undefined_payload(decls[of.local].ty).is_some()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^nullable_field_of_undefined.rs$"],
    snapshots: true,
  },
  {
    name: "held-value-field-given-box",
    breaks:
      "a struct literal's nullable field of what may be `undefined` is given a local's box as it is (rd645)",
    file: "src/lower/mir/nested_options.rs",
    find: "        match bindings::is_held(self.tcx, field) && !local.is_some_and(|place| self.held_nested(state, place, 0)) {\n",
    replace: "        match bindings::holds_nested(self.tcx, field) && !local.is_some_and(|place| self.held_nested(state, place, 0)) {\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_field_of_undefined.rs$"],
  },
];
