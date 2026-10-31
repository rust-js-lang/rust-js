// Mutations of src/lower/std_types/clock.rs (ADR 0447).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "clock-shift-adds",
    breaks: "`t - d` of a time adds `d`",
    file: "src/lower/std_types/clock.rs",
    find: "Expr::bin(if sub { Op::Sub } else { Op::Add }, t, d)",
    replace: "Expr::bin(Op::Add, t, d)",
    tests: ["test/corpus.test.ts", "-t", "^clock.rs$"],
  },
  {
    name: "clock-elapsed-backwards",
    breaks:
      "`t.elapsed()` is the time from now to `t`, an `Err` of a `SystemTime`'s",
    file: "src/lower/std_types/clock.rs",
    find: "ClockOp::Elapsed => since(now(), arg(), false),",
    replace: "ClockOp::Elapsed => since(arg(), now(), false),",
    tests: ["test/corpus.test.ts", "-t", "^clock.rs$"],
  },
  {
    name: "clock-checked-saturates",
    breaks:
      "`checked_duration_since` of an earlier `Instant` is `Some(0ns)`, not `None`",
    file: "src/lower/std_types/clock.rs",
    find: '(true, true) => "$instantCheckedSince",',
    replace: '(true, true) => "$instantSince",',
    tests: ["test/corpus.test.ts", "-t", "^clock.rs$"],
  },
];
