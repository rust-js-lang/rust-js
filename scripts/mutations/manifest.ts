// Mutations of src/manifest.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "manifest-any-version-read",
    breaks: "a library's manifest of another version is read as this one's, its fields taken for what they aren't",
    file: "src/manifest.rs",
    find: "        if manifest.version != VERSION {\n",
    replace: "        if false {\n",
    tests: ["test/publication.test.ts", "-t", "another version"],
  },
  {
    name: "manifest-fingerprint-other",
    breaks: "the native compiler fingerprints a file otherwise than the WASI host, and each takes the other's output for edited",
    file: "src/manifest.rs",
    find: "    let hash = bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {",
    replace: "    let hash = bytes.iter().fold(0xcbf29ce484222324_u64, |hash, byte| {",
    tests: ["test/publication.test.ts"],
  },
];
