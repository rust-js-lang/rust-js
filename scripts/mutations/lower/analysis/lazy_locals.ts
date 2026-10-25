// Mutations of src/lower/analysis/lazy_locals.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "lazy-local-calls-unseen",
    breaks: "a thread-local whose initializer calls the crate's function is made as its module loads, before what runs ahead of its first use",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "                    ty::InstanceKind::Item(id) => id.is_local(),\n",
    replace: "                    ty::InstanceKind::Item(id) => false && id.is_local(),\n",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_made_where_used.rs$"],
  },
  {
    name: "lazy-local-panics-unseen",
    breaks: "a thread-local whose initializer may panic is made as its module loads, so its panic is the module's",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "        TerminatorKind::Assert { .. } | TerminatorKind::Call { target: None, .. } => true,\n",
    replace: "        TerminatorKind::Assert { .. } | TerminatorKind::Call { target: None, .. } => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_made_where_used.rs$"],
  },
  {
    name: "lazy-local-drops-unseen",
    breaks: "a thread-local whose initializer drops a value with the crate's destructor is made as its module loads",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "                .is_some_and(|adt| adt.did().is_local() && adt.has_dtor(tcx))\n",
    replace: "                .is_some_and(|adt| false && adt.did().is_local() && adt.has_dtor(tcx))\n",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_made_where_used.rs$"],
  },
  {
    name: "lazy-local-pointer-unseen",
    breaks: "a thread-local whose initializer calls a fn pointer of the crate's function is made as its module loads, before what runs ahead of its first use",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "                    .is_none_or(|may_be| may_be.iter().any(|&(def_id, args)| runs(def_id, args))),\n",
    replace: "                    .is_none_or(|may_be| false && may_be.iter().any(|&(def_id, args)| runs(def_id, args))),\n",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_made_where_used.rs$"],
  },
  {
    name: "lazy-local-pointer-any",
    breaks: "a thread-local whose initializer calls a fn pointer of a binding's function alone, the scheduler's `getCurrentTime`'s `Date.now`, is made where it's first used, `$local`, not its module's `let`",
    file: "src/lower/analysis/lazy_locals.rs",
    find: "                None if ty.is_fn_ptr() => (pointers.may_be(tcx, ty))\n",
    replace: "                None if false && ty.is_fn_ptr() => (pointers.may_be(tcx, ty))\n",
    tests: ["test/port-scheduler.test.ts"],
  },
];
