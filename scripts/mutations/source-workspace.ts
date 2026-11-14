// Actual worker execution retains source-file and workspace-alias isolation.
import type {Mutation} from "../mutations";
export const mutations: Mutation[] = [
  {name: "source-workspace-shares-files", breaks: "mutant edits change the unchanged source through linked files", file: "scripts/source-workspace.ts", find: 'copyFileSync(from, to, constants.COPYFILE_FICLONE);', replace: 'symlinkSync(from, to);', tests: ["test/source-mutations.test.ts", "-t", "source mutation workspaces isolate"]},
  {name: "source-workspace-uses-original-alias", breaks: "workspace package aliases import original sources instead of the mutated copy", file: "scripts/source-workspace.ts", find: 'symlinkSync(join(workspace, path), to);', replace: 'symlinkSync(join(root, path), to);', tests: ["test/source-mutations.test.ts", "-t", "source mutation workspaces isolate"]},
];
