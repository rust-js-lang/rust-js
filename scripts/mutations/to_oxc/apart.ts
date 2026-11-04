// Mutations of src/to_oxc/apart.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "import-after-later-code-keeps-blank",
    breaks:
      "a `js::import!` written above the `use`s, a blank line above it, prints that blank line between the imports it's after, `import \"./App.css\";` apart from the vite-react example's imports (rd666)",
    file: "src/to_oxc/apart.rs",
    find: "        if start != 0 && body[i - 1].span().start > top && matches!(body[i], Statement::ImportDeclaration(_)) {\n",
    replace: "        if false && body[i - 1].span().start > top && matches!(body[i], Statement::ImportDeclaration(_)) {\n",
    tests: ["test/lowering-comments-items.test.ts", "-t", "comments above a js::import"],
  },
  {
    name: "import-blank-of-one-use-dropped",
    breaks:
      "the blank line above a `use` that makes two imports, `use crate::api::{self, Failure};`, is dropped, the first's code past the second's start (rd666)",
    file: "src/to_oxc/apart.rs",
    find: "        if start != 0 && body[i - 1].span().start > top && matches!(body[i], Statement::ImportDeclaration(_)) {\n",
    replace: "        if start != 0 && code_end > top && matches!(body[i], Statement::ImportDeclaration(_)) {\n",
    tests: ["test/pilot.test.ts", "-t", "committed JS"],
  },
  {
    name: "top-level-blank-untold",
    breaks: "the formatter sets apart what the Rust writes together, `const EMPTY = {};` and the `js::on_load!` right below it, `if (..)` across lines, by ADR 0303's rule",
    file: "src/to_oxc/apart.rs",
    find: "        told[i] = true;\n",
    replace: "",
    tests: ["test/lowering-comments.test.ts", "-t", "a blank line is where the Rust has one"],
  },
  {
    name: "top-level-gap-blank-unwritten",
    breaks: "a blank line before a type with no JS isn't before the function after it, `pub struct Unit;` then `pub fn after_a_type`",
    file: "src/to_oxc/apart.rs",
    find: "        if blank && !had {\n",
    replace: "        if blank && !had && false {\n",
    tests: ["test/lowering-comments.test.ts", "-t", "a blank line is where the Rust has one"],
  },
  {
    name: "one-line-told",
    breaks: "the functions of one line, the pilot's `#[derive(Debug, Serialize, Deserialize)]`, are taken as written together, with no blank line between",
    file: "src/to_oxc/apart.rs",
    find: "        if lines.len() < 2 {\n",
    replace: "        if lines.len() < 1 {\n",
    tests: ["test/pilot.test.ts", "-t", "the pilot's committed JS is what rust-js writes"],
  },
];
