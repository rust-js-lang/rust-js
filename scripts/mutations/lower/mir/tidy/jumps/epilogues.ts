// Mutations of src/lower/mir/tidy/jumps/epilogues.rs: a block MIR made whose
// `break`s go on to its drops and a `return`, each `break` that tail (#2215).
import type { Mutation } from "../../../../../mutations";

const keeps = "lower::mir::tidy::jumps::epilogues::tests::a_break_keeps_its_jump_where_a_copy_would_differ";

export const mutations: Mutation[] = [
  {
    name: "epilogue-copied-past-a-try",
    breaks: "a `break` inside a `try` in the block takes the block's drops, which then run before the `try`'s `finally`, or under its `catch`, where they ran after it (#2215)",
    file: "src/lower/mir/tidy/jumps/epilogues.rs",
    find: "    if tried || declared",
    replace: "    if declared",
    unitTests: keeps,
  },
  {
    name: "epilogue-copied-under-a-shadow",
    breaks: "a `break` takes a tail that reads a name the block declares, which the copy then reads instead of the one the tail read (#2215)",
    file: "src/lower/mir/tidy/jumps/epilogues.rs",
    find: "    if tried || declared.iter().any(|name| js::mentions_of(&tail, name) > 0) {",
    replace: "    if tried {",
    unitTests: keeps,
  },
  {
    name: "epilogue-copies-what-a-finally-holds",
    breaks: "a `break` takes drops a `finally` may hold, which the drop scope's one `try { .. } finally { .. }` around the block holds once, a guard's, now written in each way out (#2215)",
    file: "src/lower/mir/tidy/jumps/epilogues.rs",
    find: "        !crate::lower::drops::finally_holds(&drops) && before",
    replace: "        before",
    unitTests: keeps,
  },
  {
    name: "epilogue-consts-not-copied",
    breaks: "a `break` doesn't take a tail of `const`s of what reads alike written twice and a `return`, `const a = fhi; const b = bhi; return [lo, ..];`, so std's `size_hint` of a `Flatten` keeps its `bb18:` (#2215)",
    file: "src/lower/mir/tidy/jumps/epilogues.rs",
    find: "        !before.is_empty() && before.iter().all(declared)\n",
    replace: "        false\n",
    unitTests: "lower::mir::tidy::jumps::epilogues::tests::a_break_takes_a_tail_of_consts",
  },
  {
    name: "epilogue-const-of-a-call-copied",
    breaks: "a `break` takes a tail of a `const` of a call, which each copy then calls where it's written (#2215)",
    file: "src/lower/mir/tidy/jumps/epilogues.rs",
    find: "StmtKind::Const(_, value) if super::decisions::reads_alike_twice(value));",
    replace: "StmtKind::Const(..));",
    unitTests: "lower::mir::tidy::jumps::epilogues::tests::a_break_takes_a_tail_of_consts",
  },
  {
    name: "epilogue-return-alone-copied",
    breaks: "a `break` takes a tail of a `return` alone, which other passes copy only where it reads better: 349 snapshots worse (#2215)",
    file: "src/lower/mir/tidy/jumps/epilogues.rs",
    find: "        !before.is_empty() && before.iter().all(declared)\n",
    replace: "        before.iter().all(declared)\n",
    unitTests: keeps,
  },
];
