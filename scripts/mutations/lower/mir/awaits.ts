// Mutations of src/lower/mir/awaits.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

const snapshot = ["test/snapshots.test.ts", "-t", "async"];
// Both, the snapshot's file first, by one pattern: `bun test` reads one
// `-t`, the last, and with `--bail` the first file's failure ends the run.
// A mutant whose JS of examples/fetch.rs is wrong fails compiler.test's
// setup, a failure of no test's name, so the snapshot's runs first, and is
// named (checks 43996182737, 43999199934, inconclusive).
const awaitedOrSnapshot = ["test/snapshots.test.ts", "test/compiler.test.ts", "-t", "async: the generated JS is its snapshot|async code becomes async functions and await"];

export const mutations: Mutation[] = [
  {
    name: "await-value-unread",
    breaks:
      "what an `.await` gives is thrown away, and the variable it was to be read into is undefined",
    file: "src/lower/mir/awaits.rs",
    find: "        let destination = match read {\n",
    replace: "        let destination = match read.filter(|_| false) {\n",
    tests: awaitedOrSnapshot,
  },
  {
    name: "await-pending-taken-for-ready",
    breaks:
      "the switch's arm for `Pending`, not `Ready`, is where an `.await` goes on",
    file: "src/lower/mir/awaits.rs",
    find: "(value == 0).then_some(target)",
    replace: "(value == 1).then_some(target)",
    tests: snapshot,
  },
  {
    name: "await-false-edge-ready",
    breaks:
      "a `Ready` arm past a match's false edge is taken for its block, and what's awaited isn't read",
    file: "src/lower/mir/awaits.rs",
    find: "            while let TerminatorKind::FalseEdge { real_target, .. } = body.basic_blocks[ready].terminator().kind\n                && body.basic_blocks[ready].statements.is_empty()\n",
    replace:
      "            while let TerminatorKind::FalseEdge { real_target, .. } = body.basic_blocks[ready].terminator().kind\n                && false\n",
    tests: awaitedOrSnapshot,
  },
];
