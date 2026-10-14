// Mutations of src/publish.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "publish-stale-kept",
    breaks: "a module removed from the crate leaves its file, and its declarations, behind",
    file: "src/publish.rs",
    find: "            for path in stale {\n                std::fs::remove_file(path)?;\n            }\n",
    replace: "            for _ in stale {}\n",
    tests: ["test/publication.test.ts", "-t", "removing modules"],
  },
  {
    name: "publish-unchanged-rewritten",
    breaks: "a file whose bytes are the same is written again, and a watcher reloads what didn't change",
    file: "src/publish.rs",
    find: "        .filter(|a| !std::fs::read(&a.path).is_ok_and(|bytes| bytes == a.bytes))\n",
    replace: "        .filter(|_| true)\n",
    tests: ["test/publication.test.ts", "-t", "unchanged is left alone"],
  },
];
