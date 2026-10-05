// Mutations of src/jsx_syntax/parser.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "props-macro-not-allowed",
    breaks: "a component's props `macro` writes `#[rust_js::jsx]` on an expression, which its expansion may not",
    file: "src/jsx_syntax/parser.rs",
    find: "        format!(\"#[allow_internal_unstable(stmt_expr_attributes)] macro {ident} {body}\"),\n",
    replace: "        format!(\"macro {ident} {body}\"),\n",
    tests: ["test/crates.test.ts","-t","component of another crate"],
  },
  {
    name: "struct-update-refused",
    breaks: "\`<Image src=.. {..Default::default()} />\` is an error, named props and a base",
    file: "src/jsx_syntax/parser.rs",
    find: "                struct_update = matches!(tokens.get(0), Some(TokenTree::Token(t, _)) if t.kind == TokenKind::DotDot);",
    replace: "                struct_update = false;",
    tests: ["test/jsx.test.ts", "-t", "named props and the rest from a base"],
  },
  {
    name: "struct-update-captured",
    breaks: "a component's named props, base and children are bound in a \`match\` first, which the JS keeps",
    file: "src/jsx_syntax/parser.rs",
    find: "            || (spread.is_some() && has_children && !struct_update)",
    replace: "            || (spread.is_some() && has_children)",
    tests: ["test/jsx.test.ts", "-t", "named props and the rest from a base"],
  },
];
