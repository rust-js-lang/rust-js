// Mutations of src/jsx_syntax/formatting.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "format-tags-unseen",
    breaks: "the formatter takes `<Comp>` for a component, and refuses the attributes after its spread",
    file: "src/jsx_syntax/formatting.rs",
    find: "        self.layout.tags.extend(tags_of(&kind));\n",
    replace: "",
    tests: ["test/format.test.ts", "-t", "tag value"],
  },
  {
    name: "format-nested-jsx-unformatted",
    breaks: "a `jsx!` in a prop's Rust, `editor={jsx! { <Editor .. /> }}`, isn't laid out, and keeps whatever indentation it had",
    file: "src/jsx_syntax/formatting.rs",
    find: "                        parser::formatted(sess, inner.clone(), t.span.to(span.close), at + 4, self)?;\n",
    replace: "",
    tests: ["test/format.test.ts", "-t", "JSX formatter aligns nested props and callbacks and is idempotent"],
  },
  {
    name: "format-skip-ignored",
    breaks: "`#[rustfmt::skip]` is ignored, and the JSX of what it marks is laid out anyway",
    file: "src/jsx_syntax/formatting.rs",
    find: "        .any(|attr| attr.path_matches(&[Symbol::intern(\"rustfmt\"), Symbol::intern(\"skip\")]))",
    replace: "        .any(|attr| attr.path_matches(&[Symbol::intern(\"rustfmt\"), Symbol::intern(\"skip\")]) && false)",
    tests: ["test/format.test.ts", "-t", "JSX formatter preserves literal contents, comments, other macros and skipped items"],
  },
  {
    name: "format-foreign-jsx-parsed",
    breaks: "another crate's `foreign::jsx!` is read as rust-js's JSX, and the formatter refuses a file whose macro isn't its grammar",
    file: "src/jsx_syntax/formatting.rs",
    find: "                    if name.as_str() == \"jsx\"\n                        && !matches!(i.checked_sub(1).and_then(|at| tokens.get(at)), Some(TokenTree::Token(t, _)) if t.kind == TokenKind::PathSep)\n                    {",
    replace: "                    if name.as_str() == \"jsx\" {",
    tests: ["test/format.test.ts", "-t", "JSX formatter preserves literal contents, comments, other macros and skipped items"],
  },
  {
    name: "format-errors-printed",
    breaks: "a file the formatter can't read, as JSX whose tags don't match, is printed as if laid out, which a caller writing stdout over the file keeps",
    file: "src/jsx_syntax/formatting.rs",
    find: "        if sess.dcx().has_errors().is_none() {\n            self.output = Some(visitor.layout.finish());",
    replace: "        if true {\n            self.output = Some(visitor.layout.finish());",
    tests: ["test/format.test.ts", "-t", "formatter check is read-only and invalid input prevents selected-file writes"],
  },
  {
    name: "continued-line-flat",
    breaks: "a statement's continued line is at its statement's indent",
    file: "src/jsx_syntax/formatting.rs",
    find: "let at = self.mark(tree.span(), if starts || block { indent } else { indent + 4 });",
    replace: "let at = self.mark(tree.span(), indent);",
    tests: ["test/format.test.ts", "-t", "continued lines"],
  },
  {
    name: "block-line-continued",
    breaks: "a block that opens a line, after a condition of several, is 4 in",
    file: "src/jsx_syntax/formatting.rs",
    find: "let at = self.mark(tree.span(), if starts || block { indent } else { indent + 4 });",
    replace: "let at = self.mark(tree.span(), if starts { indent } else { indent + 4 });",
    tests: ["test/format.test.ts", "-t", "continued lines"],
  },
  {
    name: "statement-never-ends",
    breaks: "every line after a statement's first is 4 in",
    file: "src/jsx_syntax/formatting.rs",
    find: "                block || matches!(tree, TokenTree::Token(t, _) if matches!(t.kind, TokenKind::Semi | TokenKind::Comma));",
    replace: "                false;",
    tests: ["test/format.test.ts", "-t", "continued lines"],
  },
];
