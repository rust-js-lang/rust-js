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
  {
    name: "path-iter-as-ancestors",
    breaks: "a path's `iter()` gives its ancestors, not its components",
    file: "src/lower/std_types/path.rs",
    find: '            PathOp::Iter => "$pathParts",\n',
    replace: '            PathOp::Iter => "$pathAncestors",\n',
    tests: ["test/corpus.test.ts", "-t", "^path_methods.rs$"],
  },
  {
    name: "path-given-component-raw",
    breaks:
      "a `Component` given as a path is its object, which `$pathJoin` throws on",
    file: "src/lower/std_types/path.rs",
    find: "match self.recognition().is_path_component(ty) {",
    replace: "match false {",
    tests: ["test/corpus.test.ts", "-t", "^path_components.rs$"],
  },
];
