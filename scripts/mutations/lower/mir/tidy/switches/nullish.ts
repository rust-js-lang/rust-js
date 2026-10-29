// Mutations of src/lower/mir/tidy/switches/nullish.rs (ADR 0629).
import type { Mutation } from "../../../../../mutations";

const someSwitch = ["test/lowering-switch-symbols.test.ts", "-t", "a match of symbols give is given"];

export const mutations: Mutation[] = [
  {
    name: "some-switch-rest-break",
    breaks: "a loop's `break` in the `_` arm's statements goes in the `default`, where it leaves the `switch`, not the loop (#221)",
    file: "src/lower/mir/tidy/switches/nullish.rs",
    find: "    if at + 1 == body.len() || js::breaks(&body[at + 1..], None) {\n",
    replace: "    if at + 1 == body.len() {\n",
    tests: someSwitch,
  },
];
