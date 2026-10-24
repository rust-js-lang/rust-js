// Mutations of src/lower/mir/patterns.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "classify-some-local-unknown",
    breaks: "what `classify(Some(value))` gives isn't known to be there, so a test of its object leaves out `null`",
    file: "src/lower/mir/patterns.rs",
    find: "                } if destination.projection.is_empty() && calls.contains(fn_span) => Some(destination.local),\n",
    replace: "                } if false && calls.contains(fn_span) => Some(destination.local),\n",
    tests: ["test/lowering-switches.test.ts", "-t", "undefined and null"],
  },
  {
    name: "mir-payload-of-for-loop",
    breaks: "a `for` loop's item, its desugaring's `Some(x)`, is read in place, so the loop is MIR's `while (true)`",
    file: "src/lower/mir/patterns.rs",
    find: "                    if !p.span.is_desugaring(rustc_span::DesugaringKind::ForLoop) =>\n",
    replace: "                    if true =>\n",
    tests: ["test/lowering-loops.test.ts", "-t", "loops from MIR read as a person writes them"],
  },
  {
    "name": "mir-null-none-past-other-writes",
    "breaks": "an `Option` a call sets too, `and_then`'s `undefined`, is given to a nullable field as it is",
    "file": "src/lower/mir/patterns.rs",
    "find": "none && writes == locals.writes[local] && ",
    "replace": "none && ",
    "tests": ["test/lowering-options.test.ts", "-t", "a nullable field of Some or None by a branch"]
  },
  {
    "name": "let-pattern-deref-unpeeled",
    "breaks": "a `let` pattern through a reference isn't taken apart where it's made",
    "file": "src/lower/mir/patterns.rs",
    "find": "            while let thir::PatKind::Deref { subpattern, .. } = &pattern.kind {\n                pattern = subpattern;\n            }\n",
    "replace": "",
    "tests": ["test/jsx-children.test.ts", "test/jsx-evaluation-order.test.ts", "test/jsx-events.test.ts", "test/jsx-props.test.ts", "test/jsx-react-dev.test.ts", "test/jsx-types.test.ts", "test/jsx.test.ts"]
  },
  {
    "name": "let-pattern-ref-binding-unseen",
    "breaks": "a `let` pattern's binding by reference, `&(*_2).0`, isn't one it takes apart",
    "file": "src/lower/mir/patterns.rs",
    "find": "                    | Rvalue::Ref(_, BorrowKind::Shared, from),\n",
    "replace": ",\n",
    "tests": ["test/jsx-children.test.ts", "test/jsx-evaluation-order.test.ts", "test/jsx-events.test.ts", "test/jsx-props.test.ts", "test/jsx-react-dev.test.ts", "test/jsx-types.test.ts", "test/jsx.test.ts"]
  },
  {
    "name": "let-pattern-through-deref-unseen",
    "breaks": "a `let` pattern's field read through a reference, `(*_2).0`, isn't one it takes apart",
    "file": "src/lower/mir/patterns.rs",
    "find": "                && derefs.iter().all(|p| matches!(p, PlaceElem::Deref))\n",
    "replace": "                && derefs.is_empty()\n",
    "tests": ["test/jsx-children.test.ts", "test/jsx-evaluation-order.test.ts", "test/jsx-events.test.ts", "test/jsx-props.test.ts", "test/jsx-react-dev.test.ts", "test/jsx-types.test.ts", "test/jsx.test.ts"]
  },
  {
    "name": "mir-props-with-drops-whole",
    "breaks": "props with a destructor are taken whole, `{ label, rest }`, their `Rest` a field of its own, not `...rest`",
    "file": "src/lower/mir/patterns.rs",
    "find": "            if self.has_drops(pat.ty) && !self.binds_dropped_fields(pat) {",
    "replace": "            if self.has_drops(pat.ty) {",
    "tests": ["test/jsx-children.test.ts", "test/jsx-evaluation-order.test.ts", "test/jsx-events.test.ts", "test/jsx-props.test.ts", "test/jsx-react-dev.test.ts", "test/jsx-types.test.ts", "test/jsx.test.ts"]
  },
  {
    "name": "mir-arm-binding-held",
    "breaks": "an arm binding the value whole, `x if x.len() > 1`, from MIR holds `const x = s;`",
    "file": "src/lower/mir/patterns.rs",
    "find": "            thir::PatKind::Binding { .. } => Some(arm.pattern.span),\n",
    "replace": "            thir::PatKind::Binding { .. } => None::<rustc_span::Span>,\n",
    "tests": ["test/lowering-matches.test.ts", "-t", "a string's or-pattern from MIR"]
  },
  {
    name: "nullable-result-some-coalesced",
    breaks: "a `#[rust_js::nullable]` function's `Some(x)` is `x ?? null`, where it's made a value and is one",
    file: "src/lower/mir/patterns.rs",
    find: "                    true => true,\n",
    replace: "                    true => false,\n",
    tests: ["test/bindings.test.ts", "-t", "a nullable function's None is null"],
  },
  {
    name: "registered-reads-all",
    breaks: "every value read as the registry's symbol is the value itself, where the program tells its `None` too",
    file: "src/lower/mir/patterns.rs",
    find: "                    && reads.contains(fn_span)\n",
    replace: "",
    tests: ["test/lowering-symbols.test.ts", "-t", "a read of one it tells is checked"],
  },
  {
    name: "thrown-written-again",
    breaks: "a variable a downcast of an unwind's payload sets, then sets to `None`, is a `Some` wherever it's tested",
    file: "src/lower/mir/patterns.rs",
    find: "                } if destination.projection.is_empty() && locals.writes[destination.local] == 1 => {\n                    let (def_id, generic_args) = fn_def(func.ty(&body.local_decls, tcx))?;\n",
    replace: "                } if destination.projection.is_empty() => {\n                    let (def_id, generic_args) = fn_def(func.ty(&body.local_decls, tcx))?;\n",
    tests: ["test/lowering-unwinds.test.ts", "-t", "every unwind"],
  },
  {
    name: "thrown-tried-breaks",
    breaks: "`?` of what an unwind's payload's `&JsError` is returns where what was thrown is falsy",
    file: "src/lower/mir/patterns.rs",
    find: "        thrown.extend(tried);\n",
    replace: "",
    tests: ["test/lowering-unwinds.test.ts", "-t", "every unwind"],
  },
  {
    name: "held-binding-a-variable",
    breaks: "what a guard that holds binds is a variable of its own, `value$1 = value`, set in each alternative (ADR 0774)",
    file: "src/lower/mir/patterns.rs",
    find: "        for &(bound, of) in &bindings {\n            let read",
    replace: "        for &(bound, of) in &bindings[..0] {\n            let read",
    tests: ["test/lowering-labels.test.ts", "-t", "a block the program labeled that ends a case"],
  },
  {
    name: "held-arms-tested",
    breaks: "each alternative of an arm whose guard holds goes through the guard's test, so the `match` is no `switch` (ADR 0774)",
    file: "src/lower/mir/patterns.rs",
    find: "            if past_guard {\n                arms.insert(start, at);",
    replace: "            if past_guard && false {\n                arms.insert(start, at);",
    tests: ["test/lowering-labels.test.ts", "-t", "a block the program labeled that ends a case"],
  },
  {
    name: "promoted-some-maybe-none",
    breaks: "an `==` of an `Option` and a promoted `&Some(C)`, never `None`, is `==`, not the `===` a person writes",
    file: "src/lower/mir/patterns.rs",
    find: "                        && super::queries::promoted_some(self.tcx, &promoted[at]).is_some() =>\n",
    replace: "                        && false =>\n",
    tests: ["test/lowering-symbols.test.ts", "-t", "an == of a value read as the registry's symbol"],
  },
  {
    name: "reborrowed-some-maybe-none",
    breaks: "a reborrow of a reference to a `Some`, `&*r`, isn't known a `Some`, so its `==` is `==`",
    file: "src/lower/mir/patterns.rs",
    find: "                    if of.projection.is_empty() || of.projection[..] == [mir::ProjectionElem::Deref] =>\n",
    replace: "                    if of.projection.is_empty() =>\n",
    tests: ["test/lowering-symbols.test.ts", "-t", "an == of a value read as the registry's symbol"],
  },
];
