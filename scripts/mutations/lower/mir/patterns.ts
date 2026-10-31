// Mutations of src/lower/mir/patterns.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "null-none-copy-ungiven",
    breaks:
      "`let owner = None;` given to a nullable field is `undefined` where it's made, and `owner ?? null` where it's given (ADR 0781)",
    file: "src/lower/mir/patterns.rs",
    find: "                *given.entry(of).or_default() += 1;\n",
    replace: "",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "a None only nullable fields and parameters read",
    ],
  },
  {
    name: "null-none-given-optional",
    breaks:
      "`let label = None;` given to an optional nullable field is `undefined` where it's made, and `label ?? null` where it's given (ADR 0810, amended)",
    file: "src/lower/mir/patterns.rs",
    find: "                    && bindings::is_nullable(self.tcx, &fields[field])\n",
    replace:
      "                    && bindings::null_where_made(self.tcx, &fields[field])\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "an optional nullable field",
    ],
  },
  {
    name: "null-none-param-ungiven",
    breaks:
      "`let owner = None;` given to a binding's nullable parameter is `undefined` where it's made (ADR 0781)",
    file: "src/lower/mir/patterns.rs",
    find: "                let named = !nullable.is_empty() && bindings::takes_nullable(self.tcx, def_id, i);\n",
    replace:
      "                let named = !nullable.is_empty() && bindings::takes_nullable(self.tcx, def_id, i) && false;\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "a None only nullable fields and parameters read",
    ],
  },
  {
    name: "null-none-copy-read-elsewhere",
    breaks:
      "a `None` copied to a variable read elsewhere is `null`, so `let other = owner;` returns `null`, not `undefined` (ADR 0781)",
    file: "src/lower/mir/patterns.rs",
    find: "            if locals.writes[copy] == 1 && given.get(&copy).is_some_and(|&n| n == locals.reads[copy]) {\n",
    replace: "            if locals.writes[copy] == 1 {\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "a None only nullable fields and parameters read",
    ],
  },
  {
    name: "classify-some-local-unknown",
    breaks:
      "what `classify(Some(value))` gives isn't known to be there, so a test of its object leaves out `null`",
    file: "src/lower/mir/patterns.rs",
    find: "                } if destination.projection.is_empty() && calls.contains(fn_span) => Some(destination.local),\n",
    replace:
      "                } if false && calls.contains(fn_span) => Some(destination.local),\n",
    tests: ["test/lowering-switches.test.ts", "-t", "undefined and null"],
  },
  {
    name: "mir-payload-of-for-loop",
    breaks:
      "a `for` loop's item, its desugaring's `Some(x)`, is read in place, so the loop is MIR's `while (true)`",
    file: "src/lower/mir/patterns.rs",
    find: "                    if !p.span.is_desugaring(rustc_span::DesugaringKind::ForLoop) =>\n",
    replace: "                    if true =>\n",
    tests: [
      "test/lowering-loops.test.ts",
      "-t",
      "loops from MIR read as a person writes them",
    ],
  },
  {
    name: "let-pattern-deref-unpeeled",
    breaks:
      "a `let` pattern through a reference isn't taken apart where it's made",
    file: "src/lower/mir/patterns.rs",
    find: "            while let thir::PatKind::Deref { subpattern, .. } = &pattern.kind {\n                pattern = subpattern;\n            }\n",
    replace: "",
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
    name: "let-pattern-ref-binding-unseen",
    breaks:
      "a `let` pattern's binding by reference, `&(*_2).0`, isn't one it takes apart",
    file: "src/lower/mir/patterns.rs",
    find: "                    | Rvalue::Ref(_, BorrowKind::Shared, from),\n",
    replace: ",\n",
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
    name: "let-pattern-through-deref-unseen",
    breaks:
      "a `let` pattern's field read through a reference, `(*_2).0`, isn't one it takes apart",
    file: "src/lower/mir/patterns.rs",
    find: "                && derefs.iter().all(|p| matches!(p, PlaceElem::Deref))\n",
    replace: "                && derefs.is_empty()\n",
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
    name: "mir-props-with-drops-whole",
    breaks:
      "props with a destructor are taken whole, `{ label, rest }`, their `Rest` a field of its own, not `...rest`",
    file: "src/lower/mir/patterns.rs",
    find: "            if self.has_drops(pat.ty) && !self.binds_dropped_fields(pat) {",
    replace: "            if self.has_drops(pat.ty) {",
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
    name: "mir-arm-binding-held",
    breaks:
      "an arm binding the value whole, `x if x.len() > 1`, from MIR holds `const x = s;`",
    file: "src/lower/mir/patterns.rs",
    find: "            thir::PatKind::Binding { .. } => Some(arm.pattern.span),\n",
    replace:
      "            thir::PatKind::Binding { .. } => None::<rustc_span::Span>,\n",
    tests: [
      "test/lowering-matches.test.ts",
      "-t",
      "a string's or-pattern from MIR",
    ],
  },
  {
    name: "nullable-result-some-coalesced",
    breaks:
      "a `#[rust_js::nullable]` function's `Some(x)` is `x ?? null`, where it's made a value and is one",
    file: "src/lower/mir/patterns.rs",
    find: "                    true => true,\n",
    replace: "                    true => false,\n",
    tests: [
      "test/bindings.test.ts",
      "-t",
      "a nullable function's None is null",
    ],
  },
  {
    name: "registered-reads-all",
    breaks:
      "every value read as the registry's symbol is the value itself, where the program tells its `None` too",
    file: "src/lower/mir/patterns.rs",
    find: "                    && reads.contains(fn_span)\n",
    replace: "",
    tests: [
      "test/lowering-symbols.test.ts",
      "-t",
      "a read of one it tells is checked",
    ],
  },
  {
    name: "thrown-written-again",
    breaks:
      "a variable a downcast of an unwind's payload sets, then sets to `None`, is a `Some` wherever it's tested",
    file: "src/lower/mir/patterns.rs",
    find: "                } if destination.projection.is_empty() && locals.writes[destination.local] == 1 => {\n                    let (def_id, generic_args) = fn_def(func.ty(&body.local_decls, tcx))?;\n",
    replace:
      "                } if destination.projection.is_empty() => {\n                    let (def_id, generic_args) = fn_def(func.ty(&body.local_decls, tcx))?;\n",
    tests: ["test/lowering-unwinds.test.ts", "-t", "every unwind"],
  },
  {
    name: "thrown-tried-breaks",
    breaks:
      "`?` of what an unwind's payload's `&JsError` is returns where what was thrown is falsy",
    file: "src/lower/mir/patterns.rs",
    find: "        thrown.extend(tried);\n",
    replace: "",
    tests: ["test/lowering-unwinds.test.ts", "-t", "every unwind"],
  },
  {
    name: "held-binding-a-variable",
    breaks:
      "what a guard that holds binds is a variable of its own, `value$1 = value`, set in each alternative (ADR 0790)",
    file: "src/lower/mir/patterns.rs",
    find: "        for &(bound, of) in bindings {\n            let read",
    replace:
      "        for &(bound, of) in &bindings[..0] {\n            let read",
    tests: [
      "test/lowering-labels.test.ts",
      "-t",
      "a block the program labeled that ends a case",
    ],
  },
  {
    name: "held-arms-tested",
    breaks:
      "each alternative of an arm whose guard holds goes through the guard's test, so the `match` is no `switch` (ADR 0790)",
    file: "src/lower/mir/patterns.rs",
    find: "            if past_guard {\n                arms.insert(start, at);",
    replace:
      "            if past_guard && false {\n                arms.insert(start, at);",
    tests: [
      "test/lowering-labels.test.ts",
      "-t",
      "a block the program labeled that ends a case",
    ],
  },
  {
    name: "promoted-some-maybe-none",
    breaks:
      "an `==` of an `Option` and a promoted `&Some(C)`, never `None`, is `==`, not the `===` a person writes",
    file: "src/lower/mir/patterns.rs",
    find: "                        && super::queries::promoted_some(self.tcx, &promoted[at]).is_some() =>\n",
    replace: "                        && false =>\n",
    tests: [
      "test/lowering-symbols.test.ts",
      "-t",
      "an == of a value read as the registry's symbol",
    ],
  },
  {
    name: "reborrowed-some-maybe-none",
    breaks:
      "a reborrow of a reference to a `Some`, `&*r`, isn't known a `Some`, so its `==` is `==`",
    file: "src/lower/mir/patterns.rs",
    find: "                    if of.projection.is_empty() || of.projection[..] == [mir::ProjectionElem::Deref] =>\n",
    replace: "                    if of.projection.is_empty() =>\n",
    tests: [
      "test/lowering-symbols.test.ts",
      "-t",
      "an == of a value read as the registry's symbol",
    ],
  },
  {
    name: "null-nones-calls-unwritten",
    breaks:
      "a call's `Option` only a nullable field reads is `?? null` where it's read, not `null` where it's made",
    file: "src/lower/mir/patterns.rs",
    find: "                *none = true;\n",
    replace: "                *none = true;\n                *writes += 1;\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "a nullable function's result is written to a nullable place as it is",
    ],
  },
  {
    name: "null-nones-clones-written",
    breaks:
      "a nullable field's clone given to one is written `?? null` where it's made, `message.id ?? null`",
    file: "src/lower/mir/patterns.rs",
    find: "            if !cloned\n",
    replace: "            if true\n",
    tests: [
      "test/lowering-options.test.ts",
      "-t",
      "a nullable field given a nullable field's copy",
    ],
  },
  {
    name: "null-nones-own-param-call-coalesced",
    breaks:
      "a call's `Option` only a function's own nullable parameter reads is written `?? null`, `mapIntoArray(init(payload) ?? null, ..)`, where its body reads either (ADR 0861)",
    file: "src/lower/mir/patterns.rs",
    find: "                            && (!called.contains(&local) || stored.contains(&local))\n",
    replace: "",
    tests: ["test/lowering-nullable.test.ts", "-t", "own nullable parameter"],
  },
  {
    name: "ended-borrow-of-any-let",
    breaks:
      "a `let` of a `Cell`'s value, `let index = current.index.get();`, is taken for one that ends a borrow, its name lost to its place's read",
    file: "src/lower/mir/patterns.rs",
    find: "                if initializer.and_then(borrow_cloned) == Some(binding) {\n",
    replace: "                if initializer.is_some() {\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_let_read_in_place.rs$"],
    snapshots: true,
  },
  {
    name: "guard-lets-unseen",
    breaks:
      "no `let` is a drop guard's, so its `try` starts at its first statement, a comment after the `let` before it (ADR 0865, amended)",
    file: "src/lower/mir/patterns.rs",
    find: "                    && adt.has_dtor(self.tcx)\n",
    replace: "                    && false\n",
    tests: [
      "test/lowering-drop-guards.test.ts",
      "-t",
      "a guard's try starts at its let",
    ],
  },
  {
    name: "nullable-items-made-undefined",
    breaks:
      "a `None` given where a nullable field's items are made, `create_lane_map(None)`, is `undefined`, not upstream's `createLaneMap(null)`",
    file: "src/lower/mir/patterns.rs",
    find: "            *given.entry(option).or_default() += 1;\n",
    replace: "            let _ = option;\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "Vec of Options holds null",
    ],
  },
  {
    name: "nullable-init-given-undefined",
    breaks:
      "a `#[rust_js::nullable]` thread-local's generic constructor is given `undefined` for `None`, `createCursor(undefined)`, not upstream's `createCursor(null)` (ADR 0586, amended)",
    file: "src/lower/mir/patterns.rs",
    find: "                    made && inputs.get(i).is_some_and(|input| matches!(input.kind(), ty::Param(_)));\n",
    replace:
      "                    made && inputs.get(i).is_some_and(|_| false);\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "a generic constructor makes",
    ],
  },
  {
    name: "nullable-init-gives-any-parameter",
    breaks:
      "a `#[rust_js::nullable]` thread-local's constructor is given `null` for an `Option` its own parameter's type is, which isn't a type parameter's value, `pair(null, null)` (ADR 0586, amended)",
    file: "src/lower/mir/patterns.rs",
    find: "                    made && inputs.get(i).is_some_and(|input| matches!(input.kind(), ty::Param(_)));\n",
    replace: "                    made && inputs.get(i).is_some();\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "a generic constructor makes",
    ],
  },
  {
    name: "nullable-init-gives-any-call",
    breaks:
      "a call a `#[rust_js::nullable]` thread-local's initializer makes on the way, not the one whose value it is, is given `null` for `None`, `id(null)` (ADR 0586, amended)",
    file: "src/lower/mir/patterns.rs",
    find: "            let made = destination.local == mir::RETURN_PLACE\n",
    replace: "            let made = true\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "a generic constructor makes",
    ],
  },
  {
    name: "payload-test-unfolded",
    breaks:
      "a `match` of an `Option`'s constants tests `sub != null` first, its `_` arm reached twice, a labeled block `bb1:` around it",
    file: "src/lower/mir/patterns.rs",
    find: "                tests.insert(block, some);\n",
    replace: "",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^option_constants_switch.rs$"],
  },
  {
    name: "payload-test-of-none-elsewhere",
    breaks:
      'a `match` whose `None` goes to its own arm is folded, `None` taken as `Some(_)`\'s, "other"',
    file: "src/lower/mir/patterns.rs",
    find: "                && payload_targets.otherwise() == none\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^option_constants_switch.rs$"],
  },
  {
    name: "nullable-return-not-a-nullable-read",
    breaks:
      "a `let` of `None` a nullable function returns isn't read by a nullable place, so it starts `undefined` and is returned `?? null`",
    file: "src/lower/mir/patterns.rs",
    find: "                if returns_nullable && assign.0.local == mir::RETURN_PLACE {\n",
    replace:
      "                if false && returns_nullable && assign.0.local == mir::RETURN_PLACE {\n",
    tests: [
      "test/lowering-nullable-reads.test.ts",
      "-t",
      "a nullable parameter, a nullable thread-local",
    ],
  },
  {
    name: "some-in-a-temporary-not-a-write",
    breaks:
      "`found = Some(Rc::clone(&n))`, made in a temporary MIR moves in, isn't a write of a `Some`, so `found`'s `None` stays `undefined`",
    file: "src/lower/mir/patterns.rs",
    find: "                made.entry(copy).or_default().0 += 1;\n",
    replace: "",
    tests: [
      "test/lowering-nullable-reads.test.ts",
      "-t",
      "a nullable parameter, a nullable thread-local",
    ],
  },
  {
    name: "ended-borrow-of-another-name",
    breaks:
      "a `let` that ends a borrow named otherwise than the field it reads, React's `const pendingFirst = pendingQueue.next; baseQueue.next = pendingFirst;`, loses the program's name to the place's read (ADR 0866, amended)",
    file: "src/lower/mir/patterns.rs",
    find: "                if initializer.and_then(borrow_cloned) == Some(binding) {\n",
    replace:
      "                if initializer.and_then(borrow_cloned).is_some() {\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_let_read_in_place.rs$"],
    snapshots: true,
  },
  {
    name: "nones-of-a-reassigned-parameter",
    breaks:
      "a parameter the body assigns `None` is taken for one whose `None` is only `undefined`, so a JS caller's `null` is `Some` (a wrong answer, ADR 0298, amended)",
    file: "src/lower/mir/patterns.rs",
    find: "                && local.as_usize() > body.arg_count\n",
    replace: "",
    tests: ["test/lowering-option-truth.test.ts", "-t", "None it can be"],
  },
  {
    name: "nones-of-a-copy-of-what-js-gave",
    breaks:
      "a copy of a parameter, or of what's written any other way, is taken to hold a `None` that's only `undefined`, so `copy === undefined` of JS's `null` is false (a wrong answer, ADR 0298, amended)",
    file: "src/lower/mir/patterns.rs",
    find: "                        Write::Of(of) => held.get(&of).copied().unwrap_or(Err(())),\n",
    replace:
      "                        Write::Of(of) => held.get(&of).copied().unwrap_or(Ok(Some(js::Known::NoneUndefined))),\n",
    tests: ["test/lowering-option-truth.test.ts", "-t", "None it can be"],
  },
  {
    name: "nones-through-a-mut-borrow",
    breaks:
      "an `Option` borrowed whole as `&mut`, `match &mut class_name`, is taken to hold only what its own writes give, where what's written through the borrow may be any `None` (ADR 0298, amended)",
    file: "src/lower/mir/patterns.rs",
    find: "                    unknown.insert(of.local);\n                }\n                _ => {}\n",
    replace: "                }\n                _ => {}\n",
    tests: [
      "test/lowering-written-branches.test.ts",
      "-t",
      "an if a branch of which adds to its variable is as written",
    ],
  },
  {
    name: "nones-of-any-call",
    breaks:
      "what a call of the crate's gives, `found(items, 3)`, which may be what JS gave, is taken to hold a `None` that's only `undefined` (ADR 0298, amended)",
    file: "src/lower/mir/patterns.rs",
    find: "                        _ => {\n                            unknown.insert(destination.local);\n                        }\n",
    replace:
      "                        _ => writes.entry(destination.local).or_default().push(Write::Made(Some(js::Known::NoneUndefined))),\n",
    tests: ["test/corpus.test.ts", "-t", "^option_none_kinds.rs$"],
    snapshots: true,
  },
  {
    name: "nested-options-boxed",
    breaks:
      "an `Option<&Option<Rc<Fiber>>>` made and compared where it's held is boxed, `previousInstance = $some(instance)`, where upstream's is `previousInstance = instance` (ADR 1017)",
    file: "src/lower/mir/patterns.rs",
    find: "            nested: self.mir_nested(body, locals),\n",
    replace: "            nested: Default::default(),\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_held_as_payload.rs$"],
    snapshots: true,
  },
  {
    name: "nested-option-mentions-unchecked",
    breaks:
      "an `Option` of an `Option` given to a function is held as its payload too, so the function, which boxes, reads its `Some(None)`, `null`, as `None` (ADR 1017)",
    file: "src/lower/mir/patterns.rs",
    find: "                (nested.contains(&local) || refs.contains_key(&local)) && !allowed.contains(&(at, local))\n",
    replace:
      "                (nested.contains(&local) || refs.contains_key(&local)) && false\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_held_as_payload.rs$"],
  },
  {
    name: "nested-option-copies-unlinked",
    breaks:
      "an `Option` of an `Option` copied to one given to a function is held as its payload while the copy is boxed (ADR 1017)",
    file: "src/lower/mir/patterns.rs",
    find: "                if out.contains(&x) || out.contains(&y) {\n",
    replace: "                if out.contains(&x) && out.contains(&y) {\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_held_as_payload.rs$"],
  },
  {
    name: "nested-option-is-some-boxed",
    breaks:
      "`last.is_some()` of an `Option` of an `Option` held where it's made keeps its box, `$some(item)`",
    file: "src/lower/mir/patterns.rs",
    find: "                        through(&a.node)\n",
    replace: "                        None\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_tests.rs$"],
    snapshots: true,
  },
  {
    name: "nested-option-some-none-eq-boxed",
    breaks:
      "`last == Some(None)` of an `Option` of an `Option` held where it's made keeps its box, `$eq(last, { $someNone: 0 })`",
    file: "src/lower/mir/patterns.rs",
    find: "                            (Some(a), None) if self.nested_some_none(body, &b.node) => Some(a),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_tests.rs$"],
    snapshots: true,
  },
  {
    name: "getter-subject-not-held",
    breaks:
      "an `if let` of a JS-made struct's field tests it by one read and binds it by another, a getter run twice (#430)",
    file: "src/lower/mir/patterns.rs",
    find: "        if held.is_empty() {\n            return None;\n        }\n",
    replace:
      "        if !held.is_empty() {\n            return None;\n        }\n",
    tests: ["test/lowering-moves.test.ts", "-t", "JS made is read as often"],
  },
  {
    name: "getter-subject-fake-borrow-read",
    breaks:
      "a guard's fake borrow of a JS-made struct's field held where it's tested reads its getter again, `view.padding;` (#430)",
    file: "src/lower/mir/patterns.rs",
    find: "                                .is_none_or(|elem| matches!(elem, PlaceElem::Downcast(..)))",
    replace:
      "                                .is_some_and(|elem| matches!(elem, PlaceElem::Downcast(..)))",
    tests: ["test/lowering-moves.test.ts", "-t", "JS made is read as often"],
  },
  {
    name: "getter-subject-within-another-match",
    breaks:
      "an `if let` of a JS-made struct's field in the arm of one of the same field isn't held, its getter read twice (#430)",
    file: "src/lower/mir/patterns.rs",
    find: "                h.subject == span\n                    && h.place.local",
    replace: "                h.place.local",
    tests: ["test/lowering-moves.test.ts", "-t", "JS made is read as often"],
  },
  {
    name: "getter-subject-other-match-region",
    breaks:
      "a read of a held JS-made struct's field outside its match's patterns is read through its borrow (#430)",
    file: "src/lower/mir/patterns.rs",
    find: "                            && (span == h.subject || h.patterns.iter().any(|p| p.contains(span)))\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^subject_of_a_struct_js_made_read_once.rs$",
    ],
    snapshots: true,
  },
  {
    name: "nested-option-keyed-read-out",
    breaks:
      "a keyed read of a value a collection holds `None` of as `null` doesn't write a local held as its payload, so it's `$someGet`'s box, `$eq(.., { $someNone: 0 })` (ADR 0884, amended)",
    file: "src/lower/mir/patterns.rs",
    find: "                && bindings::reads_a_nullable_value(self.tcx, def_id, args)\n            {\n                allowed.insert((body.terminator_loc(block), to));",
    replace:
      "                && bindings::reads_a_nullable_value(self.tcx, def_id, args)\n                && false\n            {\n                allowed.insert((body.terminator_loc(block), to));",
    tests: ["test/js-collections.test.ts", "-t", "a Map's value is any value"],
  },
];
