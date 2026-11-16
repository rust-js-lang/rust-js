// The latent survivors' tracking issue (scripts/survivors-issue.ts, ADR
// 0093, amended): a push's run with none must pass, not fail main's check.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "latent-none-fails",
    breaks: "a push's run with no latent survivor, whose shards upload no `latent.json`, fails the `survivors` job on the missing directory, and main's check with it",
    file: "scripts/survivors-issue.ts",
    find: "  if (!existsSync(dir)) return [];\n",
    replace: "",
    tests: ["test/survivors.test.ts", "-t", "no latent survivor has nothing to add"],
  },
  {
    name: "latent-none-asks-the-issue",
    breaks: "a push's run with no latent survivor asks GitHub for the issue and writes it again, for nothing to add",
    file: "scripts/survivors-issue.ts",
    find: '  if (mode === "latent" && found.length === 0) {',
    replace: "  if (false) {",
    tests: ["test/survivors.test.ts", "-t", "no latent survivor has nothing to add"],
  },
];
