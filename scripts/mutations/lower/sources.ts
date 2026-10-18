// Mutations of src/lower/sources.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "block-comment-unread",
    breaks: "a `/* .. */` comment of the Rust isn't in the JS",
    file: "src/lower/sources.rs",
    find: "            } => Some(doc_style),\n",
    replace: "            } => None,\n",
    tests: ["test/lowering.test.ts", "-t", "a comment is in the JS where"],
  },
  {
    name: "comment-js-reads-otherwise",
    breaks: "a block comment Rust nests is written in a function's JS, which ends it at its first `*/`",
    file: "src/lower/sources.rs",
    find: "            && crate::comments::is_one_in_js(written)\n",
    replace: "            && !written.is_empty()\n",
    tests: ["test/lowering.test.ts", "-t", "a comment is in the JS where"],
  },
  {
    name: "item-span-macro-call",
    breaks: "a `thread_local!` static's `const` is where the macro is called, and a doc comment of its is lost",
    file: "src/lower/sources.rs",
    find: "            Some(name) if span.from_expansion() && !name.from_expansion() => self.span(name),\n",
    replace: "            Some(_) if false => self.span(span),\n",
    tests: ["test/lowering.test.ts", "-t", "a comment is in the JS where"],
  },
];
