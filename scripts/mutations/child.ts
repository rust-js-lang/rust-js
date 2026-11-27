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
  {
    name: "own-peak-spawned-by-the-asker",
    breaks: "a command's peak memory is measured from the process that asks it, which Linux says held at least that process's memory: a compile of 0.27 GB said to hold 1.2 GB in CI's test job (8b4a77c53's check)",
    file: "test/child.ts",
    find: "  if (p.exitCode !== 0) throw new Error(`measuring ${cmd.join(\" \")}: ${p.stderr.toString()}`);\n  return JSON.parse(p.stdout.toString()) as Peak;\n",
    replace:
      "  const own = Bun.spawnSync(cmd, { cwd, stdout: \"ignore\", stderr: \"pipe\", timeout });\n  return { code: own.exitCode, stderr: own.stderr.toString(), maxRSS: own.resourceUsage.maxRSS, spawner: p.pid && process.pid };\n",
    tests: ["test/child.test.ts", "-t", "a command's peak memory is its own"],
  },
  {
    name: "check-crashed-start-after-preload",
    breaks: "a crash once the file's preload ran, its tests running, is run again as Bun's start's, hiding a real crash (#2472)",
    file: "test/child.ts",
    find: "signal !== null && crashSignals.has(signal) && !stopped && !started",
    replace: "signal !== null && crashSignals.has(signal) && !stopped",
    tests: ["test/timing.test.ts", "-t", "Bun crashed before its first test"],
  },
  {
    name: "check-crashed-start-any-exit",
    breaks: "a test file's run that exited unstarted with no signal, a failure of its own, is run again as a crash (#2472)",
    file: "test/child.ts",
    find: "signal !== null && crashSignals.has(signal) && !stopped && !started",
    replace: "(signal === null || crashSignals.has(signal)) && !stopped && !started",
    tests: ["test/timing.test.ts", "-t", "Bun crashed before its first test"],
  },
];
