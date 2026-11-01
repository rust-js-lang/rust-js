// Mutations of src/lower/bindings/paths.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

const tests = ["test/lowering-binding-paths.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "quoted-key-named",
    breaks:
      "`window['$RT']` is read as `window.$RT`, which Closure's ADVANCED mode renames, so the server's inline scripts no longer find it",
    file: "src/lower/bindings/paths.rs",
    find: "            Step::Quoted(key) => Expr::index(object, Expr::str(key)),\n",
    replace: "            Step::Quoted(key) => Expr::member(object, key),\n",
    tests,
  },
  {
    name: "path-first-name-whole",
    breaks:
      "the global a path starts from is taken to its end, so `window['$RT']` is a variable of that name, and a local `window` isn't renamed around it",
    file: "src/lower/bindings/paths.rs",
    find: "    &path[..path.find(['.', '[']).unwrap_or(path.len())]\n",
    replace: "    &path[..path.find('.').unwrap_or(path.len())]\n",
    tests,
  },
  {
    name: "path-named-step-unchecked",
    breaks:
      "`window.$R-T` is taken, a subtraction where it's printed, not refused naming its quoted key",
    file: "src/lower/bindings/paths.rs",
    find: "            if !crate::names::is_identifier(name) {\n",
    replace: "            if name.is_empty() {\n",
    tests,
  },
  {
    name: "path-key-quotes-unchecked",
    breaks:
      "a key with a quote in it is taken where the path's written with the wrong quotes",
    file: "src/lower/bindings/paths.rs",
    find: "            if key.is_empty() || key.contains(['\\'', '\\\\']) {\n",
    replace: "            if key.is_empty() {\n",
    tests,
  },
  {
    name: "unbound-path-refused",
    breaks: "a binding's path `(0, eval)` isn't one, `js::eval` refused where it's declared",
    file: "src/lower/bindings/paths.rs",
    find: "    let path = unbound(path).unwrap_or(path);\n    let first = first_name(path);\n",
    replace: "    let first = first_name(path);\n",
    tests: ["test/js-any-values.test.ts", "-t", "eval is the global eval, called indirectly"],
  },
];
