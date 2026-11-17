// Mutations of src/lower/untagged/field_reads.rs (ADR 0093): an untagged
// enum's method that reads each variant's field (rd317, ADR 0214 amended).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "field-read-of-any-return",
    breaks:
      "a method of each variant's own field, whose JS tests the variant, `typeof parent.rootId === \"string\" ? parent.rootId : parent.tag`, is written in its callers' place too (rd317)",
    file: "src/lower/untagged/field_reads.rs",
    find: "            js::ExprKind::Var(name) if name == param => Some(Expr::var(name)),\n            _ => None,\n",
    replace: "            js::ExprKind::Var(name) if name == param => Some(Expr::var(name)),\n            _ => Some(at.clone()),\n",
    tests: ["test/corpus.test.ts", "-t", "^untagged_field_of_each_variant.rs$"],
    snapshots: true,
  },
];
