// Mutations of src/prepare/passes.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const stages = ["test/tidy-stages.test.ts", "-t", "a case's JS after tidy's first stage"];
const unfired = ["test/tidy-stages.test.ts", "-t", "fired nowhere"];

export const mutations: Mutation[] = [
  {
    name: "prepare-without-ignored-by-a-condition",
    breaks: "`--prepare-without <pass>` leaves a rewrite asked last of its conditions on, so its variant is the JS as built, checked as if it were another (ADR 1087)",
    file: "src/prepare/passes.rs",
    find: "    let on = on(pass);\n",
    replace: "    let on = true;\n",
    tests: stages,
  },
  {
    name: "prepare-without-ignored-in-place",
    breaks: "`--prepare-without <pass>` leaves a rewrite of a block or a module in place on, so its variant is the JS as built (ADR 1087)",
    file: "src/prepare/passes.rs",
    find: "pub(super) fn run<T: Clone + PartialEq>(pass: Pass, target: &mut T, rewrite: impl FnOnce(&mut T)) {\n    if !on(pass) {\n        return;\n    }\n",
    replace: "pub(super) fn run<T: Clone + PartialEq>(pass: Pass, target: &mut T, rewrite: impl FnOnce(&mut T)) {\n",
    tests: stages,
  },
  {
    name: "prepare-fired-never-recorded",
    breaks: "`--prepare-fired` writes no rewrite, so a program's preparation is never checked without one (ADR 1087)",
    file: "src/prepare/passes.rs",
    find: "    if passes.recording {\n",
    replace: "    if passes.recording && false {\n",
    tests: stages,
  },
  {
    name: "prepare-fired-in-place-unrecorded",
    breaks: "a rewrite of a block or a module that changed it isn't said to have fired, so its variant is never made, and a wrong one goes unchecked (ADR 1087)",
    file: "src/prepare/passes.rs",
    find: "    if *target != before {\n",
    replace: "    if false && *target != before {\n",
    tests: unfired,
  },
  {
    name: "prepare-fired-rewritten-unrecorded",
    breaks: "a rewrite of a value as another that changed it isn't said to have fired, `tests`' say, so its variant is never made (ADR 1087)",
    file: "src/prepare/passes.rs",
    find: "    if new != *target {\n",
    replace: "    if false {\n",
    tests: unfired,
  },
];
