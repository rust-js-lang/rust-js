// Passing jobs must not hide the child owner's observed-overrun diagnostic.
import type {Mutation} from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "check-passing-child-warning-hidden",
    breaks: "a successful job hides an observed child overrun and its job name",
    file: "scripts/checkFast.ts",
    find: "for (const warning of childWarnings(result.log)) console.warn(name + \": \" + warning);",
    replace: "void childWarnings(result.log);",
    tests: ["test/timing.test.ts","-t","child observation warning"],
  },
];
