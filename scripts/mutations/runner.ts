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
    find: "      if (value === undefined || (value === \"\" && !listFlags.includes(flag))) return [`${flag} takes a value: ${form}`];\n",
    replace: "",
    tests: ["test/mutations.test.ts", "-t", "a flag that takes a value, given none"],
  },
  {
    name: "mutations-empty-list-refused",
    breaks: "check.yml's `--also=`, no mutation retried, is refused as a flag given no value, and every mutation shard fails (c502b22f6)",
    file: "scripts/mutations.ts",
    find: " && !listFlags.includes(flag)",
    replace: "",
    tests: ["test/ci.test.ts", "-t", "the mutation shards' arguments, as check.yml writes them"],
  },
  {
    name: "mutant-setup-failure-inconclusive",
    breaks: "a mutant that fails its tests' `beforeAll`, its control having passed, is inconclusive, and the check fails: await-value-unread's examples/fetch.rs (checks 43996182737, 43999199934)",
    file: "scripts/mutations.ts",
    find: "  const failed = [...output.matchAll(/\\(fail\\) (.*)$\\n?(  \\^ .* timed out\\b)?/gm)].filter((m) => m[2] === undefined);\n",
    replace: "  const failed = [...output.matchAll(/\\(fail\\) (.*)$\\n?(  \\^ .* timed out\\b)?/gm)].filter((m) => !m[1].startsWith(\"(unnamed)\") && m[2] === undefined);\n",
    tests: ["test/mutations.test.ts", "-t", "a mutant is caught, survives, or the run says nothing of it"],
  },
  {
    name: "mutant-describe-failure-inconclusive",
    breaks: "a mutant that fails a `describe`'s body, an error between tests, its control having passed, is inconclusive",
    file: "scripts/mutations.ts",
    find: "  return p.code === 1 && failed.length + between.length > 0 ? \"caught\" : \"inconclusive\";\n",
    replace: "  return p.code === 1 && failed.length > 0 ? \"caught\" : \"inconclusive\";\n",
    tests: ["test/mutations.test.ts", "-t", "a mutant is caught, survives, or the run says nothing of it"],
  },
  {
    name: "mutant-setup-timeout-caught",
    breaks: "a setup that ran out of time, between tests, counts as caught, though it says nothing of the mutant",
    file: "scripts/mutations.ts",
    find: ".filter((m) => !/\\btimed out\\b/.test(m[1]));\n",
    replace: ";\n",
    tests: ["test/mutations.test.ts", "-t", "a mutant is caught, survives, or the run says nothing of it"],
  },
];
