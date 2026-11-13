// Mutations of src/lower/mir/patterns.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "classify-some-local-unknown",
    breaks:
      "what `classify(Some(value))` gives isn't known to be there, so a test of its object leaves out `null`",
    file: "src/lower/mir/patterns.rs",
    find: "                        .is_some_and(|call| calls.contains(&call)) =>\n",
    replace: "                        .is_some_and(|call| false && calls.contains(&call)) =>\n",
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
    name: "registered-reads-all",
    breaks:
      "every value read as the registry's symbol is the value itself, where the program tells its `None` too",
    file: "src/lower/mir/patterns.rs",
    find: "                    && crate::lower::body_queries::the_call_at(self.frame.thir, *fn_span)\n                        .is_some_and(|read| reads.contains(&read))\n",
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
    find: "            nested,\n",
    replace: "            nested: Default::default(),\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_option_held_as_payload.rs$"],
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
    name: "nulled-param-none-unsaid",
    breaks:
      "a function's own nullable parameter each call gives `null` or a value is tested `!= null`, not upstream's `!== null` (rd613)",
    file: "src/lower/mir/patterns.rs",
    find: "            held.insert(param, Ok(NoneForm::Null.trust(Writers::Exact)));\n",
    replace: "",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "each call gives null or a value"],
  },
  {
    name: "captured-payload-in-place",
    breaks:
      "React's isSignatureEqual's `boundPromise`, a pattern's binding two `.then` callbacks take, is read in place and copied apart for each, `const boundPromise = ..; const boundPromise$1 = ..;`, where upstream closes over one (rd640, ADR 0879, amended)",
    file: "src/lower/mir/patterns.rs",
    find: "                        && !captured.contains(&place.local) =>\n",
    replace: " =>\n",
    tests: ["test/lowering-moves.test.ts", "-t", "a binding closures take is one const"],
  },
  {
    name: "captured-once-held-apart",
    breaks:
      "React's RequestInstance's `environmentName == null ? () => 'Server' : .. () => environmentName`, a binding only its closure reads, is a `const` of its own, which breaks the conditional into `let tmp; if (..) { tmp = .. } else { tmp = .. }` and names every field's value before it (rd640, ADR 0879, amended)",
    file: "src/lower/mir/patterns.rs",
    find: "            .filter(|&local| locals.reads[local] > 1)\n",
    replace: "",
    tests: ["test/lowering-moves.test.ts", "-t", "a binding closures take is one const"],
  },
];
