// Mutations of src/format/jsx_shape.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "jsx-shape-lines-unjoined",
    breaks: "JSX text across lines is read as one line, its lines not joined by a space, so a layout is taken for another program (#1807)",
    file: "src/format/jsx_shape.rs",
    find: "            if i != last_text {\n                cleaned.push(' ');\n            }\n",
    replace: "",
    unitTests: "format::jsx_shape::tests::jsx_text_as_jsx_reads_it",
  },
  {
    name: "jsx-shape-strings-apart",
    breaks: "a string in braces is another child than the same JSX text, so a layout that writes one as the other is taken for another program (#1807)",
    file: "src/format/jsx_shape.rs",
    find: "                    JSXExpression::StringLiteral(string) => shape.push(Child::Text(string.value.to_string())),\n",
    replace: "                    JSXExpression::StringLiteral(_) => shape.push(Child::Node),\n",
    tests: ["test/jsx.test.ts", "-t", "keeps its whitespace"],
  },
];
