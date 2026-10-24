// Mutations of src/lower/std_types/location.rs (ADR 0427).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "location-debug-file-unquoted",
    breaks: "`{:?}` of a `Location` shows its file unquoted",
    file: "src/lower/std_types/location.rs",
    find: "                    ExprKind::Str(text) => Expr::str(format!(\"{text:?}\")),\n",
    replace: "                    ExprKind::Str(text) => Expr::str(text.clone()),\n",
    tests: ["test/corpus.test.ts", "-t", "^track_caller.rs$"],
  },
  {
    name: "location-shown-made-refused",
    breaks: "`{}` of a `Location` a call gives is refused",
    file: "src/lower/std_types/location.rs",
    find: "        if !value.reads_same() && !matches!(value.kind, ExprKind::Object(..)) {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^track_caller_traits.rs$"],
  },
];
