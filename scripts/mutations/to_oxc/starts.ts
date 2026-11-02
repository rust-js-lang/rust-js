// Mutations of src/to_oxc/starts.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "class-doc-unstarted",
    breaks: "a class's struct is no start, and its `///` doc is lost (ADR 0761)",
    file: "src/to_oxc/starts.rs",
    find: "                    starts.0.push((declared.lo, declared.hi));\n",
    replace: "",
    tests: ["test/classes.test.ts", "-t", "doc is its constructor function's"],
  },
  {
    name: "noinline-nested-unwritten",
    breaks: "a function written in a body and marked `#[inline(never)]`, React's `UnknownOwner`'s inner frame, has no `/** @noinline */` (ADR 0649, amended)",
    file: "src/to_oxc/starts.rs",
    find: "                if let StmtKind::Function(f) = &stmt.kind {\n                    note(f);\n",
    replace: "                if let StmtKind::Function(f) = &stmt.kind {\n                    let _ = f;\n",
    tests: ["test/emission.test.ts", "-t", "inline\\(never\\)"],
  },
  {
    name: "arrow-value-a-statement-too",
    breaks: "an arrow of a value's comment is its statement's too, before the arrow, and the value's",
    file: "src/to_oxc/starts.rs",
    find: "            ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body)\n                if expressions::arrow_value(body).is_some() && !methods.contains(&e.span) => {}\n",
    replace: "",
    tests: ["test/lowering-names.test.ts", "-t", "a comment is in the JS where"],
  },
  {
    name: "declared-function-body-spans-twice",
    breaks: "a comment before a statement of a function declared in a body is inside the closure that statement gives, its closures' statements found first (ADR 0882, amended)",
    file: "src/to_oxc/starts.rs",
    find: "                    functions.push(std::mem::take(&mut f.body));\n",
    replace: "                    functions.push(f.body.clone());\n",
    tests: ["test/lowering-arrows.test.ts", "-t", "where it's written"],
  },
  {
    name: "method-return-no-statement",
    breaks: "an object's method's lone `return` has no statement's place, as a concise arrow's value hasn't, so a comment its tail starts with, `maySuspendCommit(type) { // Asks .. return ..; }`, is lost (rd296)",
    file: "src/to_oxc/starts.rs",
    find: "                if expressions::arrow_value(body).is_some() && !methods.contains(&e.span) => {}\n",
    replace: "                if expressions::arrow_value(body).is_some() => {}\n",
    tests: ["test/corpus.test.ts", "-t", "^object_field_comments.rs$"],
    snapshots: true,
  },
  {
    name: "subclass-doc-unattached",
    breaks: "a class component's struct's `///` doc is nowhere, as a constructor function's is placed before its `new`",
    file: "src/to_oxc/starts.rs",
    find: "    if class.extends.is_some() {\n",
    replace: "    if false {\n",
    tests: [
      "test/class-components.test.ts",
      "-t",
      "an ES class React renders"
    ]
  },
  {
    name: "set-apart-function-no-blank-before",
    breaks: "a scope struct's method's function, declared after its value's `let`, has no blank line before it",
    file: "src/to_oxc/starts.rs",
    find: "                if apart(stmt) && !stmt.span.is_none() {\n                    starts.insert(stmt.span.lo);\n                }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "scope_struct_functions"],
    snapshots: true,
  },
  {
    name: "set-apart-function-no-blank-after",
    breaks: "what's after a scope struct's methods' functions has no blank line before it",
    file: "src/to_oxc/starts.rs",
    find: "                    starts.insert(next.span.lo);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "scope_struct_functions"],
    snapshots: true,
  },
];
