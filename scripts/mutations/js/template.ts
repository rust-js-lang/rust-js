// Mutations of src/js/template.rs: templates made, folded, broken and spliced
// (ADRs 0066, 0641).
import type { Mutation } from "../../mutations";

const pieces = ["test/lowering-strings.test.ts", "-t", "a format string of a concat! is its pieces joined by"];

export const mutations: Mutation[] = [
  {
    name: "template-piece-break-at-start",
    breaks: "a template's piece of `concat!`'s starts at its text's start, not where the one before ends",
    file: "src/js/template.rs",
    find: "                    at: last.len(),\n",
    replace: "                    at: 0,\n",
    tests: pieces,
  },
  {
    name: "template-pieces-of-text-one-string",
    breaks: "`concat!`'s pieces of a format string with nothing left to show are one string",
    file: "src/js/template.rs",
    find: "            true => Expr::written(strings),\n",
    replace: "            true => Expr::str(texts.concat()),\n",
    tests: pieces,
  },
  {
    name: "template-first-piece-lines-dropped",
    breaks: "the first of a template's `concat!` pieces, written across lines, prints its line breaks as `\\n`",
    file: "src/js/template.rs",
    find: "                0 => layout.lines = lines,\n",
    replace: "                0 => {}\n",
    tests: pieces,
  },
  {
    name: "template-across-dropped",
    breaks: "a template forgets which of its values a format argument written across lines is, so it's on the template's line however long (ADR 0641, amended)",
    file: "src/js/template.rs",
    find: "            across: Across::of(kept_across),\n",
    replace: "            across: Across::default(),\n",
    tests: ["test/lowering-strings.test.ts", "-t", "a format argument written across lines"],
  },
  {
    name: "template-literal-unfolded",
    breaks: "a literal a template shows, `${false}`, isn't written in its text",
    file: "src/js/template.rs",
    find: "                ExprKind::Bool(b) => Some(b.to_string()),\n",
    replace: "                ExprKind::Bool(b) if false => Some(b.to_string()),\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^unit_struct_order.rs$"],
  },
  {
    name: "literal-template-not-string",
    breaks: "a template with nothing left to show, all of it text, isn't a string",
    file: "src/js/template.rs",
    find: "        if kept.is_empty() {\n",
    replace: "        if false && kept.is_empty() {\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^unit_struct_order.rs$"],
  },
  {
    name: "shown-text-unspliced-across-lines",
    breaks: "`${n}` in a format string written across lines stays a template in a template, `<b>\n${`${n}`}</b>`",
    file: "src/js/template.rs",
    find: "if inner.is_plain() && (!outer_lines || inner_texts.iter().all(String::is_empty)))",
    replace: "if inner.is_plain() && !outer_lines)",
    tests: ["test/bindings.test.ts", "-t", "js::shown's text is text"],
  },
  {
    name: "applied-template-breaks-unmoved",
    breaks: "a template's `concat!` pieces, after a value's template is put in its place, break where they did in texts that are now longer",
    file: "src/js/template.rs",
    find: "        (at.text, at.at) = (text, shift + at.at);",
    replace: "        (at.text, at.at) = (text, at.at);",
    tests: ["test/lowering-strings.test.ts", "-t", "a format string of a concat! is its pieces joined by"],
  },
  {
    name: "applied-template-into-pieces-unspliced",
    breaks: "a value's template put in place of its call in a template of `concat!`'s pieces is a template in a template",
    file: "src/js/template.rs",
    find: "        let outer_lines = layout.lines || layout.breaks.iter().any(|at| at.lines);\n",
    replace: "        let outer_lines = layout.lines || !layout.breaks.is_empty();\n",
    tests: ["test/lowering-strings.test.ts", "-t", "a format string of a concat! is its pieces joined by"],
  },
];
