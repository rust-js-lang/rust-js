// Mutations of crates/rust-js-mir/src/tidy/jumps/epilogues.rs: a block MIR made whose
// `break`s go on to its drops and a `return`, each `break` that tail (#2215).
import type { Mutation } from "../../../../../mutations";

const keeps = "tidy::jumps::epilogues::tests::a_break_keeps_its_jump_where_a_copy_would_differ";

export const mutations: Mutation[] = [
  {
    name: "epilogue-copied-past-a-try",
    breaks: "a `break` inside a `try` in the block takes the block's drops, which then run before the `try`'s `finally`, or under its `catch`, where they ran after it (#2215)",
    file: "crates/rust-js-mir/src/tidy/jumps/epilogues.rs",
    find: "    if tried || shadowed",
    replace: "    if shadowed",
    unitTests: keeps,
  },
  {
    name: "epilogue-copied-under-a-shadow",
    breaks: "a `break` takes a tail that reads a name the block declares, which the copy then reads instead of the one the tail read (#2215)",
    file: "crates/rust-js-mir/src/tidy/jumps/epilogues.rs",
    find: "    if tried || shadowed || (",
    replace: "    if tried || (",
    unitTests: keeps,
  },
  {
    name: "epilogue-copies-what-a-finally-holds",
    breaks: "a `break` takes drops a `finally` may hold, which the drop scope's one `try { .. } finally { .. }` around the block holds once, a guard's, now written in each way out (#2215)",
    file: "crates/rust-js-mir/src/tidy/jumps/epilogues.rs",
    find: "        !rust_js_lower::drops::finally_holds(&drops) && before",
    replace: "        before",
    unitTests: keeps,
  },
  {
    name: "epilogue-consts-not-copied",
    breaks: "a `break` doesn't take a tail of `const`s of what reads alike written twice and a `return`, `const a = fhi; const b = bhi; return [lo, ..];`, so std's `size_hint` of a `Flatten` keeps its `bb18:` (#2215)",
    file: "crates/rust-js-mir/src/tidy/jumps/epilogues.rs",
    find: "        before.iter().all(declared)\n",
    replace: "        false\n",
    unitTests: "tidy::jumps::epilogues::tests::a_break_takes_a_tail_of_consts",
  },
  {
    name: "epilogue-const-of-a-call-copied",
    breaks: "a `break` takes a tail of a `const` of a call, which each copy then calls where it's written (#2215)",
    file: "crates/rust-js-mir/src/tidy/jumps/epilogues.rs",
    find: "StmtKind::Const(_, value) if super::decisions::reads_alike_twice(value));",
    replace: "StmtKind::Const(..));",
    unitTests: "tidy::jumps::epilogues::tests::a_break_takes_a_tail_of_consts",
  },
  {
    name: "epilogue-lone-jump-not-copied",
    breaks: "a `break` doesn't take a lone jump, `continue;`, `break;` or `return x;`, so rust_decimal's `div_remainder32` keeps its `bb37:` (#2215)",
    file: "crates/rust-js-mir/src/tidy/jumps/epilogues.rs",
    find: "        return true;\n    }\n    let Some((last, before)) = tail.split_last() else {\n",
    replace: "        return false;\n    }\n    let Some((last, before)) = tail.split_last() else {\n",
    unitTests: "tidy::jumps::epilogues::tests::a_break_takes_a_lone_jump",
  },
  {
    name: "epilogue-lone-jump-copied-into-a-loop",
    breaks: "a `break` inside a loop in the block takes a `continue;` of no label, which then goes on with that loop, not the one around the block (#2215)",
    file: "crates/rust-js-mir/src/tidy/jumps/epilogues.rs",
    find: "    if tried || shadowed || (unnamed && looped(body, &label)) {",
    replace: "    if tried || shadowed {",
    unitTests: "tidy::jumps::epilogues::tests::a_break_takes_a_lone_jump",
  },

];
