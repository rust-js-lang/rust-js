// Mutations of the guide's example runner (scripts/docs.ts): where an
// example is compiled.
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "docs-example-dir-unclaimed",
    breaks: "an example is compiled in one fixed directory each run empties, so two runs in a checkout remove each other's as they compile (#2198)",
    file: "scripts/docs.ts",
    find: "const exampleDir = (example: Example, n: number) => fixture(`docs-${relative(guide, example.page).replace(/\\//g, \"_\")}-${n}`);\n",
    replace:
      "const exampleDir = (example: Example, n: number) => { const dir = join(target, \"docs\", relative(guide, example.page).replace(/\\//g, \"_\"), String(n)); rmSync(dir, { recursive: true, force: true }); mkdirSync(dir, { recursive: true }); return dir; };\n",
    tests: ["test/docs.test.ts", "-t", "a guide run compiles in its own directory, leaving another run's"],
  },
];
