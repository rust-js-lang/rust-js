// Mutations of src/lower/recognition/open_calls.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "open-call-try-with-unknown",
    breaks: "a thread-local's `try_with` isn't known as #1285's, so its refusal doesn't name the issue",
    file: "src/lower/recognition/open_calls.rs",
    find: '    }) && tcx.item_name(id).as_str() == "try_with";\n',
    replace: '    }) && tcx.item_name(id).as_str() == "with";\n',
    tests: ["test/diagnostics.test.ts", "-t", "a thread-local's try_with"],
  },
  {
    name: "own-reader-names-no-issue",
    breaks: "a call of a crate's reader's method is refused naming no issue, where #1297 covers it",
    file: "src/lower/recognition/open_calls.rs",
    find: "        return Some(OpenCall::OwnReader);\n",
    replace: "        return None;\n",
    tests: ["test/diagnostics.test.ts", "-t", "a read of a crate's reader"],
  },
];
