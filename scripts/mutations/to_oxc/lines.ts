// Mutations of src/to_oxc/lines.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "object-end-comment-a-line-of-nothing-before",
    breaks: "a comment before an object's `}` first on its line has a line of nothing before it, which oxfmt keeps",
    file: "src/to_oxc/lines.rs",
    find: "            if head.trim().is_empty() && end.is_none() {\n",
    replace: "            if false {\n",
    tests: ["test/lowering-comments-ends.test.ts"],
  },
  {
    name: "inside-comment-map-unmoved",
    breaks: "the source map's columns after a comment inside a line aren't moved by its width, so `given` maps from the comment",
    file: "src/to_oxc/lines.rs",
    find: "            column + wider(line, 0, column),\n",
    replace: "            column,\n",
    tests: ["test/lowering-comments.test.ts", "-t", "a comment between an = and its value"],
  },
  {
    name: "split-at-line-end-blank",
    breaks: "a comment split in after a line's last `,` leaves a line of nothing after it, a blank line before the item (ADR 0708, amended)",
    file: "src/to_oxc/lines.rs",
    find: "            let ends_line = line[column as usize..].trim().is_empty();\n",
    replace: "            let ends_line = false;\n",
    tests: ["test/lowering-comments-items.test.ts", "-t", "between an array's items"],
  },
  {
    name: "operand-after-any-operator",
    breaks: "an operand's comment is after a `?`, `:` or `+` alone, so one after `&&` has no place and is lost (ADR 0444, amended)",
    file: "src/to_oxc/lines.rs",
    find: "                .rfind(operator)\n                .map(|found| (at, written, found + operator.len()))\n",
    replace: "                .rfind(['?', ':', '+'])\n                .map(|found| (at, written, found + 1))\n",
    tests: ["test/lowering-comments.test.ts", "-t", "a comment between a binary's operands"],
  },  {
    name: "if-opening-unsplit",
    breaks: "an `if (` placed with its test isn't found before its first JS, so its test's comment is split after an operator lines above, or lost",
    file: "src/to_oxc/lines.rs",
    find: "            let before = if at == line {\n                written.get(..column as usize + opening)?",
    replace: "            let before = if at == line {\n                written.get(..column as usize + opening * 0)?",
    tests: ["test/lowering-comments.test.ts", "-t", "a comment before an or's if"],
  },];
