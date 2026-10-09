// Mutations of src/lower/recognition/registry.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "registry-without-vec",
    breaks: "`Vec` isn't one of std's data structures counted, so its methods leave the ratchet unseen",
    file: "src/lower/recognition/registry.rs",
    find: '    "Vec",\n    "VecDeque",\n',
    replace: '    "VecDeque",\n',
    tests: ["test/std-coverage.test.ts"],
  },
];
