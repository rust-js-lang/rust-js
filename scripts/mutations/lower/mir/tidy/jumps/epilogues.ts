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
    find: "        || crate::lower::drops::finally_holds(&drops)\n",
    replace: "",
    unitTests: keeps,
  },
];
