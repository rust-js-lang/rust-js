// The mutation runner's own rules (scripts/mutations.ts): what a run's
// builds read, and that a run starts where no compiler was built.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "mutation-run-reads-the-live-checkout",
    breaks: "a run's builds and sites read the checkout as it's edited, not its snapshot",
    file: "scripts/mutations.ts",
    find: "  const touched = syncTree(from, to, crateFiles);\n  crateSource = to;\n",
    replace: "  const touched = syncTree(from, to, crateFiles);\n",
    tests: ["test/mutation-snapshot.test.ts"],
  },
  {
    name: "scripts-mutations-need-a-built-compiler",
    breaks: "a run of the scripts' mutations alone on a machine that built no compiler, CI's mutation shards, stops at once: \"build the read-only compiler with cargo build\"",
    file: "scripts/mutations.ts",
    find: "  return crate || harness || !given;",
    replace: "  return crate || harness;",
    tests: ["test/mutation-changes.test.ts", "-t", "builds the compiler where none is given"],
  },
  {
    name: "mutations-bare-valued-flag-runs",
    breaks: "a bare `--changed` runs every mutation, about 14,000, not the change's (#2001)",
    file: "scripts/mutations.ts",
    find: "      return value ? [] : [`${flag} takes a value: ${form}`];\n",
    replace: "      return [];\n",
    tests: ["test/mutations.test.ts", "-t", "a flag that takes a value, given none"],
  },
];
