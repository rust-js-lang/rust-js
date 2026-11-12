// Mutations of src/lower/recognition/unrecognized.rs (ADR 1128).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "recogniser-never-left-out",
    breaks: "`--without-recognizer` leaves no recogniser out: each of the net's variants is the JS a build makes, and what no recogniser takes is tried by nothing (ADR 1128)",
    file: "src/lower/recognition/unrecognized.rs",
    find: "    if caches.without_recognizers.contains(path) {\n        return None;\n    }\n",
    replace: "",
    tests: ["test/tidy-stages.test.ts", "-t", "a std function's call without its recogniser is refused"],
  },
  {
    name: "recognised-unrecorded",
    breaks: "`--recognizers-found` names no function, so the net makes no variant of any (ADR 1128)",
    file: "src/lower/recognition/unrecognized.rs",
    find: "        recognized.borrow_mut().insert(path);\n",
    replace: "",
    tests: ["test/tidy-stages.test.ts", "-t", "a std function's call without its recogniser is refused"],
  },
];
