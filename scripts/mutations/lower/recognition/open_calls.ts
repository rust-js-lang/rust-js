// Mutations of src/lower/recognition/open_calls.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "new-in-1-100-unnamed",
    breaks: "a refused call of a std item Rust 1.100 stabilized names no issue, where its tracker, #1656, says what's to come (ADR 0109, amended)",
    file: "src/lower/recognition/open_calls.rs",
    find: "            } if (version.major, version.minor) == (1, 100))\n",
    replace: "            } if (version.major, version.minor) == (0, 0))\n",
    tests: ["test/diagnostics.test.ts", "-t", "new in 1.100"],
  },
];
