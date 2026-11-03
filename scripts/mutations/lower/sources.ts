// Mutations of src/lower/sources.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "inner-doc-not-inner",
    breaks: "a `//!` isn't known for an inner doc comment, so one above a struct is lost (#486)",
    file: "src/lower/sources.rs",
    find: "            if matches!(doc_style, Some(DocStyle::Inner)) {\n",
    replace: "            if false && matches!(doc_style, Some(DocStyle::Inner)) {\n",
    tests: ["test/lowering-comments-items.test.ts", "-t", "inner doc comment is above its JS"],
  },
  {
    name: "inner-doc-not-closed-by-its-brace",
    breaks: "an inline module's `/*!` isn't ended by the module's `}`, so it's above its parent's next code (#486)",
    file: "src/lower/sources.rs",
    find: "                    && d > depth\n",
    replace: "                    && d > depth + 1\n",
    tests: ["test/lowering-comments-items.test.ts", "-t", "inner doc comment is above its JS"],
  },
  {
    name: "block-comment-unread",
    breaks: "a `/* .. */` comment of the Rust isn't in the JS",
    file: "src/lower/sources.rs",
    find: "            } => Some(doc_style),\n",
    replace: "            } => None,\n",
    tests: [
      "test/lowering-names.test.ts",
      "-t",
      "a comment is in the JS where",
    ],
  },
  {
    name: "comment-js-reads-otherwise",
    breaks:
      "a block comment Rust nests is written in a function's JS, which ends it at its first `*/`",
    file: "src/lower/sources.rs",
    find: "            && crate::comments::is_one_in_js(written)\n",
    replace: "            && !written.is_empty()\n",
    tests: [
      "test/lowering-names.test.ts",
      "-t",
      "a comment is in the JS where",
    ],
  },
  {
    name: "item-span-macro-call",
    breaks:
      "a `thread_local!` static's `const` is where the macro is called, and a doc comment of its is lost",
    file: "src/lower/sources.rs",
    find: "            Some(name) if span.from_expansion() && !name.from_expansion() => self.span(name),\n",
    replace: "            Some(_) if false => self.span(span),\n",
    tests: [
      "test/lowering-names.test.ts",
      "-t",
      "a comment is in the JS where",
    ],
  },
  {
    name: "inline-module-code-its-files",
    breaks:
      "an inline module's code is its whole file, so its file's end comment ends its JS too",
    file: "src/lower/sources.rs",
    find: "            true => self.span(item.with_hi(item.hi() - rustc_span::BytePos(1))),\n",
    replace:
      "            true => self.span(Span::with_root_ctxt(file.start_pos, file.end_position())),\n",
    tests: [
      "test/lowering-names.test.ts",
      "-t",
      "a comment before a macro's opening",
    ],
  },
  {
    name: "comments-block-of-nothing",
    breaks:
      "an `else {}` of no comment is a block of comments, kept, where it's nothing",
    file: "src/lower/sources.rs",
    find: "    (comments > 0 && !at.is_none()).then(",
    replace: "    (!at.is_none()).then(",
    tests: ["test/lowering-branches.test.ts", "-t", "an else of nothing"],
  },
  {
    name: "comments-end-only-comments",
    breaks:
      "a block's comments after its last code, `out.push(0); // Nothing to do.`, are gone, as only a block of only comments kept them (ADR 0862)",
    file: "src/lower/sources.rs",
    find: "            _ => comments = 0,\n",
    replace: "            _ => return None,\n",
    tests: [
      "test/lowering-comments.test.ts",
      "-t",
      "a comment before a None given",
    ],
  },
  {
    name: "commented-never",
    breaks:
      "a `None` given at a function's end is never taken for one a comment is before (ADR 0862)",
    file: "src/lower/sources.rs",
    find: "    (code < at.lo).then_some(at)\n",
    replace: "    (code < at.lo && false).then_some(at)\n",
    tests: [
      "test/lowering-comments.test.ts",
      "-t",
      "a comment before a None given",
    ],
  },
  {
    name: "commented-jump-uncommented",
    breaks:
      "a `break` or `continue` with no comment before it is taken for a commented one, its MIR jump placed where it's written, so a `switch` of a match whose arm breaks its loop isn't written as before (#389)",
    file: "src/lower/sources.rs",
    find: "    commented_before(jump.lo).then_some(jump)\n",
    replace: "    Some(jump)\n",
    tests: [
      "test/lowering-switch-ends.test.ts",
      "-t",
      "a match whose arm breaks its loop has no label the program didn't write",
    ],
  },
  {
    name: "tail-if-comments-unseen",
    breaks:
      "a comment ending a block after an `if`/`else`, its last, a loop's body's or an `if`'s, is printed before the loop (rd219, #428)",
    file: "src/lower/sources.rs",
    find: "                self.found.push((written, end));\n",
    replace: "",
    tests: ["test/lowering-comments-ends.test.ts", "-t", "ending a loop"],
  },
  {
    name: "safety-arena-written",
    breaks: "the source arena marks no comment the JS leaves out, a `// SAFETY:` one written (ADR 0444, amended)",
    file: "src/lower/sources.rs",
    find: "        c.written = !left_out;\n",
    replace: "",
    tests: ["test/lowering-comments-left-out.test.ts", "-t", "a SAFETY comment is left out"],
  },
  {
    name: "commented-arm-of-a-guard",
    breaks:
      "a guarded arm that does nothing, its comment above it, holds them in its body, so the guard's `case` no longer falls to the next (#508, rd619)",
    file: "src/lower/sources.rs",
    find: "                    if arm.guard.is_some()\n",
    replace: "                    if false\n",
    tests: ["test/lowering-switch-fallthrough.test.ts", "-t", "a guarded arm whose constants the next arm names falls through to its case"],
  },
  {
    name: "commented-first-arm-unmarked",
    breaks:
      "a first arm that does nothing has no marker, so a loop's `match catch_unwind(..)` whose test names the `Err` puts the comment above `Ok(()) => {}` in the `catch` (#509)",
    file: "src/lower/sources.rs",
    find: "                                TokenKind::OpenBrace if past && i == 0 => {}\n",
    replace: "",
    tests: ["test/lowering-comments-jumps.test.ts", "-t", "a comment above a loop.s Ok arm of nothing"],
  },
];
