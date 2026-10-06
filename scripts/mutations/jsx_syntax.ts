// Mutations of src/jsx_syntax.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "jsx-spans-unmarked",
    breaks: "JSX's `#[rust_js::jsx] f(..)` is an expression's attribute a stable release refuses",
    file: "src/jsx_syntax.rs",
    find: "    let span = expanded(sess, span);\n",
    replace: "",
    tests: ["test/jsx.test.ts","-t","JSX supports components across modules"],
  },
  {
    name: "jsx-call-keeps-its-jsx",
    breaks: "an expression's `jsx!` keeps its JSX, which react's macro makes its placeholder, not the element",
    file: "src/jsx_syntax.rs",
    find: "            mac.args.tokens = arm(rust, span);\n",
    replace: "",
    tests: ["test/jsx.test.ts","-t","JSX supports components across modules"],
  },
  {
    name: "tag-local-unseen",
    breaks: "a capitalized local of the function isn't a tag, and `<Comp>` asks for a component `Comp`'s macro",
    file: "src/jsx_syntax.rs",
    find: "                self.0.insert(ident.to_string());\n",
    replace: "                let _ = ident;\n",
    tests: ["test/jsx.test.ts", "-t", "tag that.s a value"],
  },
  {
    name: "export-default-dead",
    breaks: "a function only `js::export_default!` names is dead code to rustc, a warning in the editor's check",
    file: "src/jsx_syntax.rs",
    find: "                item.attrs.push(allow);\n",
    replace: "                drop(allow);\n",
    tests: ["test/editor-check.test.ts", "-t", "reads inside JSX"],
  },
];
