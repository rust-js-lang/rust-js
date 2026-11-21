// Mutations of scripts/devTimes.ts, the one writer of the machine's log of
// the dev loop's times.
import type { Mutation } from "../mutations";

const line = ["test/select.test.ts", "-t", "says its selection on its line"];

export const mutations: Mutation[] = [
  {
    name: "times-others-at-start-unkept",
    breaks: "a run's line doesn't keep how many other runs held the machine's cores as it began, so an overlapped corpus's 1,190 s reads as the code's where a quiet one took 658 s (2026-11-21)",
    file: "scripts/devTimes.ts",
    find: "  env.RUST_JS_TIMED_OTHERS = String(others());\n",
    replace: "",
    tests: line,
  },
  {
    name: "times-others-unsaid",
    breaks: "a run's line says nothing of the other runs that shared the machine's cores, so an overlapped run's time says nothing of why it doubled (2026-11-21)",
    file: "scripts/devTimes.ts",
    find: ' + (run.others ? `, other runs ${run.others[0]}/${run.others[1]}` : "")',
    replace: "",
    tests: line,
  },
];
