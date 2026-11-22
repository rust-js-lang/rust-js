// Mutations of crates/rust-js-jsx/src/resolved.rs (ADR 0093): a component's props
// predicted by the syntax pass, verified by rustc's names, replayed where
// wrong, and a replayed compile's diagnostics said once (ADR 0213, amended).
import type { Mutation } from "../../mutations";

const glob = ["test/jsx-props-resolved.test.ts", "-t", "a glob imports"];
const warned = ["test/jsx-props-resolved.test.ts", "-t", "said once"];

export const mutations: Mutation[] = [
  {
    name: "props-flip-unread",
    breaks: "a replay builds a component's props as the first compile predicted, a literal, so it's wrong again, an error where the crate compiles",
    file: "crates/rust-js-jsx/src/resolved.rs",
    find: "        true => true,\n",
    replace: "        true => false,\n",
    tests: glob,
  },
  {
    name: "props-prediction-unrecorded",
    breaks: "a literal's prediction isn't recorded, so rustc's names never verify it, and a glob's library props type stays a literal",
    file: "crates/rust-js-jsx/src/resolved.rs",
    find: "            state.predicted.insert(at, name.to_string());\n",
    replace: "            let _ = name;\n",
    tests: glob,
  },
  {
    name: "props-companion-never-owned",
    breaks: "no struct is found to own a companion, so a glob's library props type stays a literal, its flattened attributes rustc's E0560",
    file: "crates/rust-js-jsx/src/resolved.rs",
    find: "            && owns_companion(tcx, adt.did())\n",
    replace: "            && false\n",
    tests: glob,
  },
  {
    name: "replayed-diagnostics-kept",
    breaks: "a compile stopped for a replay says what it held, so a warning the replay says too is said twice",
    file: "crates/rust-js-jsx/src/resolved.rs",
    find: "    held::discard();\n",
    replace: "    held::release();\n",
    tests: warned,
  },
  {
    name: "first-compile-unheld",
    breaks: "a run's first compile says its diagnostics as it goes, so one stopped for a replay has said a warning the replay says again",
    file: "crates/rust-js-jsx/src/resolved.rs",
    find: "                false => Hold::Holding(Vec::new()),\n",
    replace: "                false => Hold::Saying(Vec::new()),\n",
    tests: warned,
  },
];
