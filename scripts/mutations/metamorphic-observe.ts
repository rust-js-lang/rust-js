// Mutations of the metamorphic net's harness for its observe rewrites
// (#2481, ADR 1104 amended): the rewrites the nightly's alone, and a rewrite
// whose every variant is refused failing. The rewriter's own Rust,
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
];
