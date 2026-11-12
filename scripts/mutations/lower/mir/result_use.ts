// Mutations of src/lower/mir/result_use.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "result-use-conversion-unasked",
    breaks: "a `Result` given on by `?` whose `From` makes another error, `At<S::Error>`, is taken as given on as it is, its error thrown unconverted (#1545)",
    file: "src/lower/mir/result_use.rs",
    find: "    if given.is_none() || given != error_of(body.return_ty()) {\n",
    replace: "    if given.is_none() {\n",
    tests: ["test/serde-protocol.test.ts"],
  },
  {
    name: "result-use-drop-taken-for-a-use",
    breaks: "the drop of a `Result` a panic's cleanup makes is taken for a look at it, so every `?` of a protocol call whose error has drop glue is written as an inspected one, `$jsonTry` round each (#1545)",
    file: "src/lower/mir/result_use.rs",
    find: "            if local == self.local && context.is_use() && !context.is_place_assignment() && !dropped {\n",
    replace: "            if local == self.local && context.is_use() && !context.is_place_assignment() {\n",
    tests: ["test/serde-protocol.test.ts"],
  },
  {
    name: "result-use-always-propagated",
    breaks: "every protocol call's `Result` is taken as given on by `?`, so an impl's `Err` arm never runs (#1545)",
    file: "src/lower/mir/result_use.rs",
    find: "        true => ResultUse::Propagated,\n        false => ResultUse::Inspected,\n",
    replace: "        true => ResultUse::Propagated,\n        false => ResultUse::Propagated,\n",
    tests: ["test/serde-protocol.test.ts"],
  },
  {
    name: "tried-write-result-held",
    breaks: "a writer's `write!(f, ..)?` of what may fail is made a value and given on by `?`, `$writeTry(..)` around its write, where the writer's own error carries it (#1524)",
    file: "src/lower/mir/result_use.rs",
    find: "    match uses.count == 1 && tried.count() == 1 {\n",
    replace: "    match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^write_result_kept_by_a_writer.rs$"],
    snapshots: true,
  },
  {
    name: "last-write-result-held",
    breaks: "a writer's last `write!` of what may fail, its own result, is made a value and given back, `$writeTry(..)` around its write, where the writer's own error carries it (#1524)",
    file: "src/lower/mir/result_use.rs",
    find: "    if destination.local == mir::RETURN_PLACE {\n        return ResultUse::Propagated;\n    }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^write_result_kept_by_a_writer.rs$"],
    snapshots: true,
  },
];
