// Mutations of src/lower/sources.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "block-comment-unread",
    breaks: "a `/* .. */` comment of the Rust isn't in the JS",
    file: "src/lower/sources.rs",
    find: "            } => Some(doc_style),\n",
    replace: "            } => None,\n",
    tests: ["test/lowering-names.test.ts", "-t", "a comment is in the JS where"],
  },
  {
    name: "comment-js-reads-otherwise",
    breaks: "a block comment Rust nests is written in a function's JS, which ends it at its first `*/`",
    file: "src/lower/sources.rs",
    find: "            && crate::comments::is_one_in_js(written)\n",
    replace: "            && !written.is_empty()\n",
    tests: ["test/lowering-names.test.ts", "-t", "a comment is in the JS where"],
  },
  {
    name: "item-span-macro-call",
    breaks: "a `thread_local!` static's `const` is where the macro is called, and a doc comment of its is lost",
    file: "src/lower/sources.rs",
    find: "            Some(name) if span.from_expansion() && !name.from_expansion() => self.span(name),\n",
    replace: "            Some(_) if false => self.span(span),\n",
    tests: ["test/lowering-names.test.ts", "-t", "a comment is in the JS where"],
  },
  {
    name: "inline-module-code-its-files",
    breaks: "an inline module's code is its whole file, so its file's end comment ends its JS too",
    file: "src/lower/sources.rs",
    find: "            true => self.span(item.with_hi(item.hi() - rustc_span::BytePos(1))),\n",
    replace: "            true => self.span(Span::with_root_ctxt(file.start_pos, file.end_position())),\n",
    tests: ["test/lowering-names.test.ts", "-t", "a comment before a macro's opening"],
  },
  {
    name: "comments-block-of-nothing",
    breaks: "an `else {}` of no comment is a block of comments, kept, where it's nothing",
    file: "src/lower/sources.rs",
    find: "    (comments > 0 && !at.is_none()).then(",
    replace: "    (!at.is_none()).then(",
    tests: ["test/lowering-branches.test.ts", "-t", "an else of nothing"],
  },
  {
    name: "comments-end-only-comments",
    breaks: "a block's comments after its last code, `out.push(0); // Nothing to do.`, are gone, as only a block of only comments kept them (ADR 0858)",
    file: "src/lower/sources.rs",
    find: "            _ => comments = 0,\n",
    replace: "            _ => return None,\n",
    tests: ["test/lowering-comments.test.ts", "-t", "a comment before a None given"],
  },
  {
    name: "commented-never",
    breaks: "a `None` given at a function's end is never taken for one a comment is before (ADR 0858)",
    file: "src/lower/sources.rs",
    find: "    (code < at.lo).then_some(at)\n",
    replace: "    (code < at.lo && false).then_some(at)\n",
    tests: ["test/lowering-comments.test.ts", "-t", "a comment before a None given"],
  },
];
