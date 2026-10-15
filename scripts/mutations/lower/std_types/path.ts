// Mutations of src/lower/std_types/path.rs (ADR 0405).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "path-relative-as-absolute",
    breaks: "`is_relative()` is whether a path is absolute",
    file: "src/lower/std_types/path.rs",
    find: "                    PathOp::IsRelative => Expr::unary(crate::js::UnaryOp::Not, rooted),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^path_methods.rs$"],
  },
];
