// A survivor's run at the base, in the run's own checkout
// (scripts/latent-survivors.ts, #2222): it leaves the checkout, and what
// the run wrote of its own, as it found them.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "base-run-past-the-checkouts-changes",
    breaks: "a survivor's run at the base checks out the base over the checkout's own changes, and its checking out back discards them (#2222)",
    file: "scripts/latent-survivors.ts",
    find: '  if (status.stdout.trim() !== "") return { results: {}, why:',
    replace: "  if (false) return { results: {}, why:",
    tests: ["test/survivors.test.ts", "-t", "leaves the run's checkout as it found it"],
  },
  {
    name: "base-run-leaves-its-results",
    breaks: "a survivor's run at the base, in the run's own checkout, leaves the base's results.json where the run's was, which CI uploads as the run's (#2222)",
    file: "scripts/latent-survivors.ts",
    find: "    for (const [name, text] of kept) writeFileSync(join(work, name), text);\n",
    replace: "",
    tests: ["test/survivors.test.ts", "-t", "leaves the run's checkout as it found it"],
  },
  {
    name: "base-run-stays-at-the-base",
    breaks: "a survivor's run at the base leaves the run's checkout at the base, so what the run does after, and CI's later steps, read the base's tree (#2222)",
    file: "scripts/latent-survivors.ts",
    find: '    await git(["checkout", "--quiet", "--force", "--detach", head]);\n',
    replace: "",
    tests: ["test/survivors.test.ts", "-t", "leaves the run's checkout as it found it"],
  },
];
