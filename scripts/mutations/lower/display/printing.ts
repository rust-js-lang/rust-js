// Mutations of src/lower/display/printing.rs (ADR 1130).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "print-while-formatting-logged",
    breaks: "a print made while a line is formatted is `console.log`'s, not the line's, so it comes before the text formatted so far (#1490)",
    file: "src/lower/display/printing.rs",
    find: "        self.runs_while_formatting()\n            .then(|| Expr::call_helper(\"$out\", vec![text.clone()]))\n",
    replace: "        (false && self.runs_while_formatting())\n            .then(|| Expr::call_helper(\"$out\", vec![text.clone()]))\n",
    tests: ["test/corpus.test.ts", "-t", "^printed_while_a_line_is_formatted.rs$"],
  },
  {
    name: "format-while-printing-in-the-line",
    breaks: "a `format!` of a writer that prints, made as a line is formatted, writes its text into the line, and the string it gives is empty (#1490)",
    file: "src/lower/display/printing.rs",
    find: "        match self.runs_while_formatting() {\n            true => Expr::call_helper(\n                \"$formatting\",",
    replace: "        match false {\n            true => Expr::call_helper(\n                \"$formatting\",",
    tests: ["test/corpus.test.ts", "-t", "^printed_while_a_line_is_formatted.rs$"],
  },
];
