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
];
