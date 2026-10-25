// Mutations of src/lower/declarations/comments.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "declaration-docs-dropped",
    breaks: "a doc comment isn't its declaration's JSDoc in a `.d.ts`, which a TypeScript caller's editor shows",
    file: "src/lower/declarations/comments.rs",
    find: "    let all: Vec<String> = written.into_iter().chain(jsdoc).collect();\n",
    replace: "    let all: Vec<String> = written.into_iter().chain(jsdoc.filter(|_| false)).collect();\n",
    tests: ["test/declarations.test.ts", "-t", "doc comments as JSDoc"],
  },
  {
    name: "declaration-comments-dropped",
    breaks: "a type's and a field's `//` comments aren't in its `.d.ts`, only its doc comment (ADR 0196, amended)",
    file: "src/lower/declarations/comments.rs",
    find: "    let written = leading_comments(tcx, def_id);\n",
    replace: "    let written: Option<String> = None;\n",
    tests: ["test/declarations-comments.test.ts", "-t", "the comments written before an item"],
  },
  {
    name: "declaration-comments-of-the-license",
    breaks: "the file's license, before its first code, is its first item's comment too, twice in the `.d.ts`",
    file: "src/lower/declarations/comments.rs",
    find: "            // Its file's comments, before its first code.\n            return None;\n",
    replace: "            break;\n",
    tests: ["test/declarations-comments.test.ts", "-t", "the comments written before an item"],
  },
  {
    name: "declaration-comments-trailing-too",
    breaks: "a comment after a field on its line, `retry_lane: Cell<Lanes>, // ..`, is the next field's",
    file: "src/lower/declarations/comments.rs",
    find: "                if !before[line..start].trim().is_empty() {\n",
    replace: "                if false && !before[line..start].trim().is_empty() {\n",
    tests: ["test/declarations-comments.test.ts", "-t", "the comments written before an item"],
  },
  {
    name: "declaration-comments-stop-at-attributes",
    breaks: "a field's comment before its `#[cfg_attr(..)]` is lost, the attribute read as code",
    file: "src/lower/declarations/comments.rs",
    find: "            (TokenKind::CloseBracket, _) => {\n",
    replace: "            (TokenKind::CloseBracket, _) if false => {\n",
    tests: ["test/declarations-comments.test.ts", "-t", "the comments written before an item"],
  },
  {
    name: "declaration-comments-blank-dropped",
    breaks: "`// TODO ..\\n\\npub type Lanes`'s blank line after the comment is gone in the `.d.ts`",
    file: "src/lower/declarations/comments.rs",
    find: "                if blank {\n",
    replace: "                if false && blank {\n",
    tests: ["test/declarations-comments.test.ts", "-t", "the comments written before an item"],
  },
];
