// Mutations of src/jsx_syntax/formatting/shape.rs, the shape of a `jsx!`
// body its layout reads (#1901, ADR 1166).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "shape-prop-name-undashed",
    breaks: "a prop's name ends at its first dash, so `aria-expanded` can't be read and its body isn't laid out",
    file: "src/jsx_syntax/formatting/shape.rs",
    find: "            while self.token(TokenKind::Minus) || self.token(TokenKind::Colon) {\n",
    replace: "            while self.token(TokenKind::Colon) {\n",
    tests: ["test/format.test.ts", "-t", "lays out a jsx! body at rustfmt's width"],
  },
  {
    name: "shape-comment-child-a-piece",
    breaks: "a `{/* first\\n second */}` child is read as a piece of Rust, whose comment's second line rustfmt indents",
    file: "src/jsx_syntax/formatting/shape.rs",
    find: "                && inner.is_empty()\n",
    replace: "                && inner.is_empty()\n                && false\n",
    tests: ["test/format.test.ts", "-t", "preserves literal contents"],
  },
];
