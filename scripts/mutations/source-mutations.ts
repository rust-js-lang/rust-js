// The source-execution boundary is itself held by real-process regressions.
import type {Mutation} from "../mutations";
export const mutations: Mutation[] = [
  {name: "source-unloaded-accepted", breaks: "unrelated passing tests falsely judge an unloaded owner", file: "scripts/source-mutations.ts", find: 'if (!sourceWasLoaded(marker, mutation.name))', replace: 'if (false)', tests: ["test/source-mutations.test.ts", "-t", "passing tests that never load"]},
  {name: "source-bad-control-accepted", breaks: "a source mutation is caught even when its unchanged control already fails", file: "scripts/source-mutations.ts", find: 'if (!changed && (!tested.passed || tested.ran === 0))', replace: 'if (false)', tests: ["test/source-mutations.test.ts", "-t", "already failing source control"]},
  {name: "source-setup-failure-kept", breaks: "failed mutation setup leaves its independently copied workspace behind", file: "scripts/source-mutations.ts", find: 'peak = Math.max(peak, bytesOf(workspace));\n      rmSync(workspace, { recursive: true, force: true });', replace: 'peak = Math.max(peak, bytesOf(workspace));', tests: ["test/source-mutations.test.ts", "-t", "failed mutation setup"]},
  {name: "source-flake-a-catch", breaks: "a source mutant's test that fails once and passes run again with it counts as a catch", file: "scripts/source-mutations.ts", find: "(tests) => test(workspace, env, tests)", replace: "async () => tested", tests: ["test/source-mutations.test.ts", "-t", "fails once and passes run again"]},
];
