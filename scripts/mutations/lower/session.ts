// Mutations of src/lower/session.rs (ADR 0093, ADR 1111).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "session-caches-outlive-it",
    breaks:
      "a lowering session's caches aren't taken away as it ends, so a second session in the process would be answered with the first's items, its crates numbered anew (ADR 1111)",
    file: "src/lower/session.rs",
    find: "        SESSION.with(|slot| slot.borrow_mut().take());\n",
    replace: "        SESSION.with(|slot| slot.borrow().is_some());\n",
    unitTests: "lower::session::tests::a_second_session_starts_with_caches_of_its_own",
  },
  {
    name: "session-caches-of-any-tcx",
    breaks: "a session's cache asked with another session's `tcx` answers with this one's, unchecked (ADR 1111)",
    file: "src/lower/session.rs",
    find: "        caches.gcx, gcx,\n",
    replace: "        gcx, gcx,\n",
    unitTests: "lower::session::tests::a_cache_asked_with_another_sessions_tcx_is_a_bug",
  },
];
