// Mutations of Crater's store (scripts/crater.ts): a sweep that skips what another run removed.
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "crater-bound-throws-on-gone",
    breaks: "Crater's store's bound throws on an entry another run removed since it was listed",
    file: "scripts/crater.ts",
    find: "      const seen = statSync(path, { throwIfNoEntry: false });\n",
    replace: "      const seen = statSync(path);\n",
    tests: ["test/support.test.ts", "-t", "a store's sweep skips an entry gone since it was listed"],
  },
];
