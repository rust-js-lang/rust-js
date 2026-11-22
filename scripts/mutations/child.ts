// Child deadlines use queued exit events, never delayed parent observation.
import type {Mutation} from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "child-timeout-before-io-drain",
    breaks: "an overdue parent timer kills before receiving the child's queued exit",
    file: "test/child.ts",
    find: "    setTimeout(() => {\n      if (exited) return;\n      timedOut = true;\n      stop();\n    }, 0);",
    replace: "    if (!exited) { timedOut = true; stop(); }",
    tests: ["test/child.test.ts","-t","completed child is never timed out"],
  },
  {
    name: "child-external-signal-is-timeout",
    breaks: "an external signal is falsely attributed to the runner's timeout",
    file: "test/child.ts",
    find: "    timedOut,\n    overflowed,",
    replace: "    timedOut: timedOut || p.signalCode != null,\n    overflowed,",
    tests: ["test/child.test.ts","-t","external SIGKILL"],
  },
  {
    name: "child-observed-overrun-unreported",
    breaks: "late observation hides its slow spot instead of warning without guessing",
    file: "test/child.ts",
    find: "const observedOverdueMs = !timedOut && observedMs > timeout ? Math.ceil(observedMs - timeout) : undefined;",
    replace: "const observedOverdueMs: number | undefined = undefined;",
    tests: ["test/child.test.ts","-t","late child observation"],
  },
  {
    name: "footprint-asked-again-after-hanging",
    breaks: "a footprint that hung is asked again at each process measured, so each job of a run waits out its limit while what it waits on stays, the corpus's 15 minutes past 45 (#2329)",
    file: "test/child.ts",
    find: "    hung.add(tool.join(\" \"));\n",
    replace: "",
    tests: ["test/child.test.ts", "-t", "a footprint that hangs is stopped past its limit"],
  },
];
