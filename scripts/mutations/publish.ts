// Mutations of crates/rust-js-emit/src/publish.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "publish-stale-kept",
    breaks: "a module removed from the crate leaves its file, and its declarations, behind",
    file: "crates/rust-js-emit/src/publish.rs",
    find: "            for path in stale {\n                std::fs::remove_file(path)?;\n            }\n",
    replace: "            for _ in stale {}\n",
    tests: ["test/publication.test.ts", "-t", "removing modules"],
  },
  {
    name: "publish-unchanged-rewritten",
    breaks: "a file whose bytes are the same is written again, and a watcher reloads what didn't change",
    file: "crates/rust-js-emit/src/publish.rs",
    find: "        .filter(|a| !std::fs::read(&a.path).is_ok_and(|bytes| bytes == a.bytes))\n",
    replace: "        .filter(|_| true)\n",
    tests: ["test/publication.test.ts", "-t", "unchanged is left alone"],
  },
  {
    name: "publish-edited-removed",
    breaks: "an older build's file a person has edited since is removed as stale",
    file: "crates/rust-js-emit/src/publish.rs",
    find: "        .filter(|old| {\n            std::fs::read(&old.file).is_ok_and(|bytes| rust_js_program::manifest::fingerprint(&bytes) == old.hash)\n        })\n",
    replace: "        .filter(|old| old.file.exists())\n",
    tests: ["test/publication.test.ts", "-t", "native: one changed since it was written"],
  },
];
