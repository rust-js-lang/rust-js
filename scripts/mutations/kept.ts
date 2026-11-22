// Mutations of the native programs' kept builds (test/kept.ts): a sweep that
// skips what another run removed, and the working set, what the machine's
// runs used, which neither a store's cap nor the budget takes (#2163, #2344).
import type { Mutation } from "../mutations";

const working = ["test/support.test.ts", "-t", "a checkout past the budget keeps its working set"];

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
  {
    name: "disk-working-set-evicted",
    breaks: "a checkout past the budget evicts its working set too, so each check's measure removes every corpus run it's about to reuse (#2163's regression: check:fast ~300 s to 630-809 s)",
    file: "test/kept.ts",
    find: "  const stale = kept.filter((entry) => entry.used < since);",
    replace: "  const stale = kept;",
    tests: working,
  },
  {
    name: "disk-last-check-unread",
    breaks: "the working set is the last day's alone, not what this checkout's last check used, so a checkout checked 31 hours ago loses what that check reused",
    file: "test/kept.ts",
    find: "  return Math.min(last, now - workingSetMs);",
    replace: "  return now - workingSetMs;",
    tests: working,
  },
  {
    name: "disk-floor-spares-working-set",
    breaks: "the disk at its floor never takes from the working set, so a full disk stops every run instead of losing a cache",
    file: "test/kept.ts",
    find: "  const floored = evict(working, { cap: Number.POSITIVE_INFINITY, free, floor: floorBytes });",
    replace: "  const floored = { removed: 0, size: working.reduce((n, entry) => n + entry.bytes, 0) };",
    tests: working,
  },
  {
    name: "kept-cap-takes-working-set",
    breaks: "a kept store's cap takes builds in use, its working set taken as stale, so native-cache's 10 GiB evicted the 11.5 GiB the Mac mini's checks used, and the next check's every corpus part stopped at its limit, a 1,018 s re-warm (#2344)",
    file: "test/kept.ts",
    find: "  const evicted = holdKept(keptBuilds(cache), { cap, free, since });",
    replace: "  const evicted = holdKept(keptBuilds(cache), { cap, free, since: Number.POSITIVE_INFINITY });",
    tests: ["test/support.test.ts", "-t", "a kept store past its cap keeps its working set"],
  },
];
