// Mutations of the native programs' kept builds (test/kept.ts): a sweep that skips what another run removed.
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "kept-prune-throws-on-gone",
    breaks: "the kept builds' prune throws on a build another run's prune removed since it was listed",
    file: "test/kept.ts",
    find: "      // One another run's prune removed since it was listed is gone.\n      const seen = statSync(path, { throwIfNoEntry: false });\n",
    replace: "      const seen = statSync(path);\n",
    tests: ["test/support.test.ts", "-t", "a store's sweep skips an entry gone since it was listed"],
  },
  {
    name: "kept-builds-listed-gone",
    breaks: "a build gone since its program was listed, another run's prune's, is kept and read, and the prune throws",
    file: "test/kept.ts",
    find: '      return seen === undefined || build.startsWith(".building-") ? [] : [{ path, bytes: bytesOf(path), used: seen.mtimeMs }];',
    replace: '      return build.startsWith(".building-") ? [] : [{ path, bytes: bytesOf(path), used: seen!.mtimeMs }];',
    tests: ["test/support.test.ts", "-t", "a store's sweep skips an entry gone since it was listed"],
  },
];
