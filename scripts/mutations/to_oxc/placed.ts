// Mutations of src/to_oxc/placed.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const tests = ["test/lowering-comments-ends.test.ts", "-t", "guard's finally written past it"];

export const mutations: Mutation[] = [
  {
    name: "copied-statement-where-written",
    breaks: "a `Drop` impl's body, in a guard's `finally` with the impl's spans, past a function of only comments, prints that function's comments there, SchedulerPostTask's handleAbortError's in runTask's (rd328)",
    file: "src/to_oxc/placed.rs",
    find: "        if !block && past(self.function, start) {\n",
    replace: "        if false && !block && past(self.function, start) {\n",
    tests,
  },
  {
    name: "copies-unseen",
    breaks: "no function's statement copied from past it is placed in it, a `Drop` impl's body printing the comments of what's between there (rd328)",
    file: "src/to_oxc/placed.rs",
    find: "        self.copied |= !matches!(it, Statement::BlockStatement(_)) && past(self.function, start);\n",
    replace: "        self.copied |= false;\n",
    tests,
  },
  {
    name: "copied-block-end-where-written",
    breaks: "a block of only comments in a `Drop` impl's body ends where it's written, past the function its body's in, and prints there the comments of a function between (rd328)",
    file: "src/to_oxc/placed.rs",
    find: "        if past(self.function, span.end) && (span.start == span.end || past(self.function, span.start)) {\n",
    replace: "        if false {\n",
    tests,
  },
  {
    name: "copied-block-placed-as-a-statement",
    breaks: "a copied block of only comments is placed where it starts, empty, its comments not before its `}`, printed in the next branch",
    file: "src/to_oxc/placed.rs",
    find: "        let block = matches!(it, Statement::BlockStatement(_));\n",
    replace: "        let block = false;\n",
    tests,
  },
  {
    name: "placed-at-a-node-start",
    breaks: "a copied statement's place is where a node starts, a call's argument, which prints its comments there, `work(// Restored. n, out)`",
    file: "src/to_oxc/placed.rs",
    find: "        self.keys.insert(kind.span().start);\n",
    replace: "",
    tests,
  },
  {
    name: "placed-past-the-function-start",
    breaks: "a copied statement is placed past where its function starts, not past what's printed before it, a block's comments printed before the function's first statement",
    file: "src/to_oxc/placed.rs",
    find: "            self.after = self.after.max(start);\n",
    replace: "",
    tests,
  },
  {
    name: "own-place-not-given",
    breaks: "a statement whose comments are at a place of its own is given none, so they're printed where it starts, above `let value;` (rd630)",
    file: "src/to_oxc/placed.rs",
    find: "            self.given[i] = Some(place);\n            *it.span_mut() = Span::new(place, place);\n",
    replace: "            self.given[i] = Some(place);\n",
    tests: ["test/lowering-comments-jumps.test.ts", "-t", "Ok arm is above its value"],
  },
];
