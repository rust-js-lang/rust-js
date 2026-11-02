// Mutations of src/lower/analysis/wholes.rs (ADR 0994, amended): which
// `f64`s are whole, whose JS text is Rust's.
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "whole-some-payload-unread",
    breaks:
      "the `Some`'s number of a nullable WebIDL integer, `Some(start) => ..`, isn't known whole (ADR 0994, amended)",
    file: "src/lower/analysis/wholes.rs",
    find: "                [PlaceElem::Downcast(_, some), PlaceElem::Field(..)] if some.as_u32() == 1 => Some(Some(read.local)),\n",
    replace: "",
    tests: ["test/webapi-bindings.test.ts", "-t", "WebIDL integer"],
  },
  {
    name: "whole-parameter-trusted",
    breaks:
      "a parameter set again to a WebIDL integer is known whole, though its caller gave `-0`, which `${n}` writes `0` (ADR 0994, amended)",
    file: "src/lower/analysis/wholes.rs",
    find: "        (local == RETURN_PLACE || local.as_usize() > body.arg_count)\n            && ",
    replace: "        ",
    tests: ["test/webapi-bindings.test.ts", "-t", "WebIDL integer"],
  },
  {
    name: "counter-past-2-53",
    breaks:
      "a counter that starts past 2^53, `1e21`, is taken for whole and shown as JS's text, `1e+21` where Rust shows its digits (rd266)",
    file: "src/lower/analysis/wholes.rs",
    find: "n.fract() == 0.0 && n.abs() <= 9007199254740992.0 && ",
    replace: "n.fract() == 0.0 && ",
    tests: ["test/corpus.test.ts", "-t", "^counter_shown_as_js_text.rs$"],
    snapshots: true,
  },
  {
    name: "counter-negative-zero",
    breaks:
      "a counter that starts at `-0` is shown as JS's text, `0` where Rust shows `-0` (rd266)",
    file: "src/lower/analysis/wholes.rs",
    find: " && !(n == 0.0 && n.is_sign_negative())",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^counter_shown_as_js_text.rs$"],
    snapshots: true,
  },
  {
    name: "counter-any-step",
    breaks:
      "a counter of any step, `big += 4e20`, is shown as JS's text, `1.2e+21` past 1e21 (rd266)",
    file: "src/lower/analysis/wholes.rs",
    find: "f64::from_bits(scalar.to_bits_unchecked() as u64) == 1.0)",
    replace:
      "f64::from_bits(scalar.to_bits_unchecked() as u64).fract() == 0.0)",
    tests: ["test/corpus.test.ts", "-t", "^counter_shown_as_js_text.rs$"],
    snapshots: true,
  },
  {
    name: "counter-of-itself-unknown",
    breaks:
      "a counter, written of itself, isn't known whole, `$displayF64(index)` (rd266)",
    file: "src/lower/analysis/wholes.rs",
    find: "                    .any(|of| of.is_some_and(|of| !found.contains(&of)))\n",
    replace: "                    .any(|of| of.is_some())\n",
    tests: ["test/corpus.test.ts", "-t", "^counter_shown_as_js_text.rs$"],
    snapshots: true,
  },
  {
    name: "whole-field-set-anything",
    breaks:
      "a `Cell<f64>` field set to what isn't whole, `big.at.set(1e21)`, is taken for whole, its text `1e+21` where Rust's is its digits (rd537)",
    file: "src/lower/analysis/wholes.rs",
    find: "                Some(CellMethod::Writes) => args.get(1).is_some_and(|value| whole_operand(tcx, &value.node, locals)),\n",
    replace: "                Some(CellMethod::Writes) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^whole_counters.rs$"],
    snapshots: true,
  },
  {
    name: "whole-field-lent-mutably",
    breaks:
      "an `f64` field lent mutably, `bump(&mut lent.n)`, which adds 1e21, is taken for whole (rd537)",
    file: "src/lower/analysis/wholes.rs",
    find: "            Rvalue::RawPtr(_, place) | Rvalue::Ref(_, BorrowKind::Mut { .. }, place)\n                if let Some(field) = field_of(tcx, body, *place) =>\n",
    replace: "            Rvalue::RawPtr(_, place) | Rvalue::Ref(_, BorrowKind::Mut { .. }, place)\n                if let Some(field) = field_of(tcx, body, *place) && false =>\n",
    tests: ["test/corpus.test.ts", "-t", "^whole_counters.rs$"],
    snapshots: true,
  },
  {
    name: "whole-public-field",
    breaks:
      "a `pub` struct's `pub` field, which another crate may write, is taken for whole (rd537)",
    file: "src/lower/analysis/wholes.rs",
    find: "                let outside = tcx.visibility(id).is_public() && tcx.visibility(field.did).is_public();\n",
    replace: "                let outside = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^whole_counters.rs$"],
    snapshots: true,
  },
  {
    name: "whole-returns-unchecked",
    breaks:
      "a function of an `f64` that gives `-0.0` is taken for whole, its text `0` where Rust's is `-0` (rd537)",
    file: "src/lower/analysis/wholes.rs",
    find: "                    gone.returns.insert(body.def_id);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^whole_counters.rs$"],
    snapshots: true,
  },
  {
    name: "whole-field-reads-unknown",
    breaks:
      "a counter's `Cell<f64>` field's `get` isn't known whole, `prefix + $displayF64(blobId)` where upstream's is `prefix + blobId` (rd537)",
    file: "src/lower/analysis/wholes.rs",
    find: "                        || (matches!(cell_own_method(tcx, def_id), Some(CellMethod::Get | CellMethod::Writes))\n",
    replace: "                        || (false\n",
    tests: ["test/corpus.test.ts", "-t", "^whole_counters.rs$"],
    snapshots: true,
  },
  {
    name: "whole-constant-held",
    breaks:
      "a struct a constant holds, `static SIGNED: Fixed = Fixed { n: -0.0 }`, whose value rustc evaluates where no write is seen, is taken for whole, its `-0` shown `0` (rd537)",
    file: "src/lower/analysis/wholes.rs",
    find: "            Field::Struct(field) => !constant.structs.contains(&tcx.parent(field)),\n",
    replace: "            Field::Struct(_) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^whole_counters.rs$"],
    snapshots: true,
  },
  {
    name: "whole-constant-fields-unwalked",
    breaks:
      "a struct a constant holds inside another's field or behind a reference, a promoted `&Promoted { n: 1e21 }`, is taken for whole (rd537)",
    file: "src/lower/analysis/wholes.rs",
    find: "            ty::Ref(_, of, _) | ty::Array(of, _) | ty::Slice(of) => self.holds(of),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^whole_counters.rs$"],
    snapshots: true,
  },
  {
    name: "whole-borrow-uses-unseen",
    breaks:
      "a field's `Cell` lent to a function, copied into an array or returned, `set_negative_zero(&given.at)`, `[&repeated.at; 2]`, `&handed.at`, is taken for unlent (rd537)",
    file: "src/lower/analysis/wholes.rs",
    find: "    uses.visit_body(body);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^whole_counters.rs$"],
    snapshots: true,
  },
  {
    name: "whole-capture-unfound",
    breaks:
      "a `Cell<f64>` a closure takes by value and counts in, react-markup's `stash_error_idx`, isn't known whole, `$displayF64(stashErrorIdx)` where upstream writes `'' + stashErrorIdx++` (rd653)",
    file: "src/lower/analysis/wholes.rs",
    find: "        ty::Closure(closure, _) => Some(Field::Capture(closure, index)),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^whole_captures.rs$"],
    snapshots: true,
  },
  {
    name: "whole-capture-literal-unchecked",
    breaks:
      "a closure's literal isn't checked, so a `Cell` it takes made of `-0.0`, or set before, is taken for whole and `0` is shown where Rust shows `-0` (rd653)",
    file: "src/lower/analysis/wholes.rs",
    find: "                    AggregateKind::Closure(closure, _) => Some(Field::Capture(closure, index)),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^whole_captures.rs$"],
    snapshots: true,
  },
  {
    name: "whole-cell-set-before-kept",
    breaks:
      "a `Cell::new(1.0)` set `-0.0` before a closure takes it, `idx.set(-0.0); move || ..`, is taken for whole and `0` is shown where Rust shows `-0` (rd653)",
    file: "src/lower/analysis/wholes.rs",
    find: "            && uses[destination.local.as_usize()] == 1\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^whole_captures.rs$"],
    snapshots: true,
  },
  {
    name: "whole-capture-lent-unseen",
    breaks:
      "a closure's `Cell` lent to a function, `set_negative_zero(&lent)`, is taken for unlent, as only a reference's type was asked whether it's a `Cell` (rd653)",
    file: "src/lower/analysis/wholes.rs",
    find: "body.local_decls[*local].ty.peel_refs()",
    replace: "body.local_decls[*local].ty",
    tests: ["test/corpus.test.ts", "-t", "^whole_(captures|counters).rs$"],
    snapshots: true,
  },
];
