// Mutations of src/manifest.rs (ADR 0093).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "manifest-build-unread",
    breaks: "a manifest's compiler build isn't read, so a library another build of one version made is taken for this one's, its facts another compiler's (ADR 1003)",
    file: "src/manifest.rs",
    find: "    #[serde(default, skip_serializing_if = \"Option::is_none\")]\n    pub build: Option<String>,\n",
    replace: "    #[serde(skip_deserializing, default = \"Compiler::this_build\", skip_serializing_if = \"Option::is_none\")]\n    pub build: Option<String>,\n}\n\nimpl Compiler {\n    fn this_build() -> Option<String> {\n        Some(BUILD.to_owned())\n    }\n",
    tests: ["test/cargo-link.test.ts", "-t", "a real Cargo dependency links"],
  },
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
