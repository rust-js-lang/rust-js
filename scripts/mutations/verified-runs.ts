// The corpus's verified runs (test/verified-runs.ts, ADR 0567): the nightly's
// share run again.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "std-reach-keyed-by-nothing",
    breaks: "a run's key has none of the std code its JS can run, so a change to a function it calls keeps a verdict of what it ran before (#2352)",
    file: "test/verified-runs.ts",
    find: "  const reach = runKey([await built.outputs[0]!.text()]);",
    replace: "  const reach = runKey([\"\"]);",
    tests: ["test/verified-runs.test.ts", "-t", "a std edit the JS can't run keeps its runs' key"],
  },
  {
    name: "std-reach-kept-unchecked",
    breaks: "a kept reach is read again though a file of the JS it bundled changed, a stale bundle's, so the JS's new calls of std are keyed by its old ones (#2352)",
    file: "test/verified-runs.ts",
    find: "    if (inputs.every(([file, hash]) => existsSync(join(dir, file)) && sha(readFileSync(join(dir, file), \"utf8\")) === hash)) {",
    replace: "    if (inputs.length >= 0) {",
    tests: ["test/verified-runs.test.ts", "-t", "a kept reach is read again only while each file of the JS it bundled is as it was"],
  },
  {
    name: "std-reach-without-harness",
    breaks: "the reach has none of what the harness calls of std, the oracle's `$isPanic`, so an edit to it keeps every verdict (#2352)",
    file: "test/verified-runs.ts",
    find: "          names.length > 0 ?",
    replace: "          false ?",
    tests: ["test/verified-runs.test.ts", "-t", "a std edit the JS can't run keeps its runs' key"],
  },
  {
    name: "shared-without-reach",
    breaks: "a run's key has none of std, so a change to what a case's JS runs of std keeps its verdict (#2352)",
    file: "test/verified-runs.ts",
    find: "  return runKey([sharedKey, await reached(entry, std)]);",
    replace: "  return runKey([sharedKey]);",
    tests: ["test/verified-runs.test.ts", "-t", "a run's key has the std code its JS reaches"],
  },
  {
    name: "verified-run-recheck-skipped",
    breaks: "the nightly's share of the corpus's verified runs isn't run again, so a verified run a key missed an input of is never seen",
    file: "test/verified-runs.ts",
    find: "  return !rechecked();\n}",
    replace: "  return true;\n}",
    tests: ["test/native-outcomes.test.ts", "-t", "verified run is run again"],
  },
];
