// Mutations of src/lower/mir/tidy/again.rs (ADR 0093): each transition of
// whether tidy's stages run again (#872).
import type { Mutation } from "../../../../mutations";

const unit = "lower::mir::tidy::again::tests::found_is_kept_until_taken_and_done_ends_the_run";

export const mutations: Mutation[] = [
  {
    name: "again-found-unrecorded",
    breaks:
      "a pass that finds tidy's stages must run again, `scopes`'s, records nothing, so they don't, and what a guard's `try` held apart stays apart (#872)",
    file: "src/lower/mir/tidy/again.rs",
    find: "            (_, true) => State::Must,\n",
    replace: "            (state, true) => state,\n",
    tests: ["test/lowering-drop-guards.test.ts", "-t", "a guard's fields are its variables"],
  },
  {
    name: "again-found-forgotten-by-a-later-pass",
    breaks:
      "a later pass of the stage that finds nothing forgets what an earlier one found, so `looped drops`' finding is lost to `drop scopes`' (#872)",
    file: "src/lower/mir/tidy/again.rs",
    find: "            (state, false) => state,\n",
    replace: "            (_, false) => State::Idle,\n",
    unitTests: unit,
  },
  {
    name: "again-taken-not-running",
    breaks:
      "the stages taken to run again aren't running, so a declared flattened state's handle written whole is left a handle, printed as a setter of the variable holding the chunk (#779, #872)",
    file: "src/lower/mir/tidy/again.rs",
    find: "                self.0.set(State::Running);\n",
    replace: "                self.0.set(State::Idle);\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_written_whole_after_its_fields.rs$"],
  },
  {
    name: "again-running-unasked",
    breaks: "a pass of a stage run again isn't told it is, so a flattened state's handle written whole is left a handle (#779, #872)",
    file: "src/lower/mir/tidy/again.rs",
    find: "        self.0.get() == State::Running\n",
    replace: "        false\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_written_whole_after_its_fields.rs$"],
  },
  {
    name: "again-done-still-running",
    breaks: "tidy's stages run again stay running once they're done (#872)",
    file: "src/lower/mir/tidy/again.rs",
    find: "        self.0.set(State::Idle);\n    }\n",
    replace: "    }\n",
    unitTests: unit,
  },
  {
    name: "again-taken-of-nothing",
    breaks: "tidy's stages run again where no pass found they must (#872)",
    file: "src/lower/mir/tidy/again.rs",
    find: "            State::Idle => false,\n",
    replace: "            State::Idle => true,\n",
    unitTests: unit,
  },
];
