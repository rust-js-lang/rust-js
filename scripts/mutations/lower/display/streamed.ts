// Mutations of src/lower/display/streamed.rs (ADR 0093): a `Debug`
// builder written as it's taken, through a writer's sink (#1533, #1557).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "derived-debug-assembled-through-a-sink",
    breaks: "a derived `Debug` of a writer that writes through a sink is made whole, so what it wrote before a field that panics or gives `Err` is lost, `Holder { n: 1, bad: ` (#1530, #1557)",
    file: "src/lower/display/streamed.rs",
    find: "            (WriteCall::Struct, [type_name, rest @ ..]) => (\n",
    replace: "            (WriteCall::Struct, [type_name, rest @ ..]) if false => (\n",
    tests: ["test/corpus.test.ts", "-t", "^(printing_derived_debug_stopped_part_way|failing_debug_part_keeps_what_was_written)\\.rs$"],
  },
  {
    name: "streamed-step-non-exhaustive-finished",
    breaks: "a streamed builder's `finish_non_exhaustive` is its `finish`, without `..`",
    file: "src/lower/display/streamed.rs",
    find: "        true => \"finishNonExhaustive\",\n",
    replace: "        true => \"finish\",\n",
    tests: ["test/corpus.test.ts", "-t", "^printing_builder_written_step_by_step\\.rs$"],
  },
  {
    name: "streamed-map-entry-writers-swapped",
    breaks: "a builder's map entry written into a writer writes its key by its value's type and its value by its key's (#1652)",
    file: "src/lower/display/streamed.rs",
    find: '                ("entryWith", vec![key, value, write_key, write_value])\n',
    replace: '                ("entryWith", vec![key, value, write_value, write_key])\n',
    tests: ["test/writer-sink.test.ts"],
  },
];
