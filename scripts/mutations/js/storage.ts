// Mutations of src/js/storage.rs (ADR 0093), moved from make.rs's with
// the code they mutate.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "nullable-null-unplaced",
    breaks: "a nullable field's `null` is where nothing is, so the source map maps `value: null`'s `null` to nothing, not the Rust's `None`",
    file: "src/js/storage.rs",
    find: "        ExprKind::Undefined | ExprKind::GivenUndefined => Some(Expr::null().or_at(made.span)),\n",
    replace: "        ExprKind::Undefined | ExprKind::GivenUndefined => Some(Expr::null()),\n",
    tests: ["test/emission.test.ts", "-t", "a nullable field's null maps to the None it is"],
  },
  {
    name: "nulled-field-coalesced",
    breaks: "what MIR says is `null` or a value already is `?? null` given to a `#[rust_js::nullable]` field",
    file: "src/js/storage.rs",
    find: "            Storage::NullForNone if nulled && !matches!(made.kind, ExprKind::Undefined | ExprKind::GivenUndefined) => {\n",
    replace: "            Storage::NullForNone if false && nulled => {\n",
    tests: ["test/corpus.test.ts", "-t", "^(refcell_clone_through_rc|weak_cycle_set_after).rs$"],
    snapshots: true,
  },
  {
    name: "null-for-none-uncoalesced",
    breaks: "what a nullable field is given that may be `undefined` is stored as it is, `undefined`, not `?? null`",
    file: "src/js/storage.rs",
    find: "                None => Expr::bin(Op::Coalesce, made, Expr::null()),\n",
    replace: "                None => made,\n",
    tests: ["test/lowering-nullable.test.ts", "-t", "where it's made and where it's written"],
  },
  {
    name: "null-for-none-objects-coalesced",
    breaks: "what's never `undefined`, an object made where it's stored, is `?? null` to a nullable field, `context.Consumer = { .. } ?? null`",
    file: "src/js/storage.rs",
    find: "        _ if !made.may_be_undefined() => Some(made.clone()),\n",
    replace: "        _ if made.is_constant() => Some(made.clone()),\n",
    tests: ["test/lowering-nullable.test.ts", "-t", "what's never undefined as it is"],
  },
  {
    name: "null-for-none-branches-coalesced",
    breaks: "a test whose branches are each stored as they are is `?? null` as a whole, `(n != null ? n + 1 : undefined) ?? null`",
    file: "src/js/storage.rs",
    find: "        ExprKind::Cond(test, yes, no) => Some(Expr {\n",
    replace: "        ExprKind::Cond(test, yes, no) if false => Some(Expr {\n",
    tests: ["test/lowering-nullable.test.ts", "-t", "what's never undefined as it is"],
  },
  {
    name: "in-place-tag-left",
    breaks: "a whole write of a flattened state sets its variant's fields but not its tag, so it still reads as the variant it was (rd380)",
    file: "src/js/storage.rs",
    find: "            let sets = props.iter().map(|prop| match prop {",
    replace: "            let sets = props.iter().skip(1).map(|prop| match prop {",
    tests: ["test/classes-flattened-state.test.ts", "-t", "made, read and replaced in place"],
  },
  {
    name: "alike-none-written-undefined",
    breaks: "a nullable thread-local nothing reads apart set `None` holds `undefined`, `resolveFamily = undefined;`, where its initializer and upstream write `null` (#665)",
    file: "src/js/storage.rs",
    find: "            (Storage::NullForNone, ExprKind::Undefined | ExprKind::GivenUndefined) => Expr::null().or_at(made.span),\n",
    replace: "",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "a nullable thread-local nothing reads apart"],
  },
  {
    name: "alike-test-branch-undefined",
    breaks: "a nullable thread-local nothing reads apart set a test's `None`, `handler.filter(|_| !debug)`, holds `undefined` in that branch, `.. ? handler : undefined`, where a nullable place's `None` is `null` (#665)",
    file: "src/js/storage.rs",
    find: "                    Box::new(self.written_alike(*no)),\n",
    replace: "                    no,\n",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "a nullable thread-local nothing reads apart"],
  },
];
