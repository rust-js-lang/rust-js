// Mutations of the metamorphic net's harness for its observe rewrites
// (#2481, ADR 1104 amended): the rewrites the nightly's alone, and a rewrite
// whose every variant is refused failing; and a list of several cases in
// one process refused, naming parts.ts (#2618). The rewriter's own Rust,
// `scripts/metamorph`, is no source the runner mutates; its tests hold it.
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "nightly-only-rewrites-sampled",
    breaks: "a push's sample runs the nightly's own rewrites, new nets whose known lists are the nightly's",
    file: "test/metamorphic.test.ts",
    find: '(named === undefined ? which === "all" || !nightlyOnly.includes(rewrite) : named.includes(rewrite))',
    replace: '(named === undefined ? true : named.includes(rewrite))',
    tests: ["test/metamorphic.test.ts", "-t", "a push's sample runs no nightly-only rewrite"],
  },
  {
    name: "every-variant-refused-unsaid",
    breaks: "a rewrite whose every variant rust-js refuses passes, though it judged nothing, as `observe-borrows`' first sweep passed 804 of 804",
    file: "test/metamorphic.test.ts",
    find: "compiled === 0 && new Set(refused.map(refusedCase)).size >= allRefusedAt",
    replace: "compiled < 0 && new Set(refused.map(refusedCase)).size >= allRefusedAt",
    tests: ["test/metamorphic.test.ts", "-t", "every variant rust-js refuses judged nothing"],
  },
  {
    name: "judged-nothing-by-variants",
    breaks: "a rewrite judged nothing where every variant of a few cases is refused, by its variants, so one case's six, `opaque`'s, a part's whole sample of it, fail the push's sample by its chance (the nightly of 2026-11-25)",
    file: "test/metamorphic.test.ts",
    find: "const refusedCase = (refusal: string) => refusal.slice(refusal.indexOf(\" \") + 1, refusal.indexOf(\": \"));\n",
    replace: "const refusedCase = (refusal: string) => refusal;\n",
    tests: ["test/metamorphic.test.ts", "-t", "a rewrite whose every variant rust-js refuses judged nothing"],
  },
  {
    name: "metamorphic-one-process-list-runs",
    breaks: "the metamorphic net runs a list of many cases in one process, one after another, 11 cases 6 min on 12 cores, where parts.ts runs them at once (#2618)",
    file: "test/metamorphic.test.ts",
    find: "if (refusedList !== undefined) throw new Error(refusedList);\n",
    replace: "",
    tests: ["test/metamorphic.test.ts", "-t", "a list of more than one case in one process is refused"],
  },
  {
    name: "metamorphic-one-process-list-of-many-unsaid",
    breaks: "a list of many cases in one process is said fine, as one of one is",
    file: "test/metamorphic.test.ts",
    find: "split < 0 || count < 2 ||",
    replace: "split < 0 || count < 1e9 ||",
    tests: ["test/metamorphic.test.ts", "-t", "a list of more than one case in one process is refused"],
  },
];
