// Mutations of test/oracle.ts (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "js-panic-unlocated",
    breaks: "a run's panic says no location where the run asks it, so where it was raised is compared with native Rust's nowhere (#2292)",
    file: "test/oracle.ts",
    find: "    if ($isPanic(e)) return located ? { panic: (e as Error).message, at: (e as Record<symbol, string | undefined>)[$raisedAt] ?? \"<unknown>\" } : { panic: (e as Error).message };\n",
    replace: "    if ($isPanic(e)) return false ? { panic: (e as Error).message, at: (e as Record<symbol, string | undefined>)[$raisedAt] ?? \"<unknown>\" } : { panic: (e as Error).message };\n",
    tests: ["test/oracle.test.ts", "-t", "a panic's location is read where asked"],
  },
  {
    name: "located-otherwise-matching-unreported",
    breaks: "a case `located-otherwise` says panics elsewhere than native's is never found raised where native's is, so the list never shrinks (#2292)",
    file: "test/oracle.ts",
    find: "  return outcomes.length > 0 && outcomes.every((outcome) => typeof outcome !== \"string\" && \"at\" in outcome && outcome.at === at);\n",
    replace: "  return false && outcomes.every((outcome) => typeof outcome !== \"string\" && \"at\" in outcome && outcome.at === at);\n",
    tests: ["test/oracle.test.ts", "-t", "a panic's location is read where asked"],
  },
];
