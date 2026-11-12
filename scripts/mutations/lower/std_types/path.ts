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
    name: "path-given-own-struct",
    breaks: "a path's method given a type of the crate's own that's an `AsRef<Path>` is given the struct, not its `as_ref()`, and `$pathJoin` throws a `TypeError` (#1388)",
    file: "src/lower/std_types/path.rs",
    find: "        let Some(target) = self.as_ref_target(callee, at) else {",
    replace: "        let Some(target) = self.as_ref_target(callee, at).filter(|_| false) else {",
    tests: ["test/corpus.test.ts", "-t", "^path_given_own_as_ref.rs$"],
  },
  {
    name: "path-given-reference-bound-unseen",
    breaks: "`Path::new(&name)`'s bound, of its `&S`, isn't found, so it's given the struct, not its `AsRef<OsStr>`'s text (#1388)",
    file: "src/lower/std_types/path.rs",
    find: "        let declared = self.signature_of(callee).0.get(at)?.peel_refs();",
    replace: "        let declared = *self.signature_of(callee).0.get(at)?;",
    tests: ["test/corpus.test.ts", "-t", "^path_given_own_as_ref.rs$"],
  },
  {
    name: "os-string-into-string-unwrapped",
    breaks: "an `OsString`'s `into_string()` is its text itself, no `Ok`, so `unwrap()` of it reads a string's `_0` (#1420)",
    file: "src/lower/std_types/path.rs",
    find: "            None if op == PathOp::IntoString => crate::js::result::ok(values.next().expect(\"the text\")),\n",
    replace: "            None if op == PathOp::IntoString => values.next().expect(\"the text\"),\n",
    tests: ["test/corpus.test.ts", "-t", "^os_string_into_string_and_bound_clone.rs$"],
  },
];
