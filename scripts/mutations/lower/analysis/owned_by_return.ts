// Mutations of src/lower/analysis/owned_by_return.rs (ADR 0317, amended):
// what a function's return alone owns of what it's given, found where
// something else may keep it.
import type { Mutation } from "../../../mutations";

const corpus = (name: string) => ["test/corpus.test.ts", "-t", `^${name}.rs$`];
const file = "src/lower/analysis/owned_by_return.rs";

export const mutations: Mutation[] = [
  {
    name: "owned-field-read-otherwise",
    breaks:
      "a field read but to call through it, a method that gives a clone of the callback, is taken to keep it alone, so called after the request is dropped the clone reads the request, where Rust's upgrade gives `None`",
    file,
    find: "        let field_kept = |adt: DefId, field: FieldIdx| kept_field(tcx, adt, field) && !read.contains(&(adt, field));\n",
    replace: "        let field_kept = |adt: DefId, field: FieldIdx| kept_field(tcx, adt, field) || !read.contains(&(adt, field));\n",
    tests: corpus("owner_handle_field_cloned"),
  },
  {
    name: "owned-field-of-a-clone",
    breaks:
      "a field of a struct that's `Clone` is taken to keep its callback alone, so a clone of the request keeps it after the request is dropped, and it reads the request, where Rust's upgrade gives `None`",
    file,
    find: "        && clone.is_some_and(|clone| trait_holds_in(tcx, typing_env, of, clone) == Some(false))\n",
    replace: "",
    tests: corpus("owner_handle_cloned_request"),
  },
  {
    name: "owned-field-public",
    breaks:
      "a public field, which another crate may clone, is taken to keep its callback alone: the library's contract says its return owns it",
    file,
    find: "        && !tcx.visibility(field.did).is_public()\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "owns its callbacks alone"],
  },
  {
    name: "owned-by-any-call",
    breaks:
      "a parameter given on to a function that keeps it apart is taken to be owned by what that returns, so called after the request is dropped the callback reads it, where Rust's upgrade gives `None`",
    file,
    find: "                _ if tcx.generics_of(id).count() == 0 && by(id, given as u32).is_some() => {\n",
    replace: "                _ if tcx.generics_of(id).count() == 0 => {\n",
    tests: corpus("owner_handle_stashed"),
  },
  {
    name: "owned-whatever-is-returned",
    breaks:
      "a function that keeps the request its callback is given to and returns another is taken to own it by what it returns, so the callback reads the one returned after it's dropped, where Rust's upgrade gives `None`",
    file,
    find: "    let all = !defs.is_empty() && defs.iter().all(|at| returned.contains(at));\n",
    replace: "    let all = !defs.is_empty() || !values.is_empty();\n",
    tests: corpus("owner_handle_returns_another"),
  },
  {
    name: "quiet-field-named",
    breaks:
      "a body that names the field a callback is kept in, all that may call it, is taken to call none, so a request that lends it to a helper reading the field is quiet and its handle's `unwrap`s are the request (rd621)",
    file,
    find: "                .all(|body| !names(body) && !unread(body))\n",
    replace: "                .all(|body| !unread(body))\n",
    tests: corpus("owner_handle_named"),
    snapshots: true,
  },
  {
    name: "quiet-unfollowed",
    breaks:
      "what rust-js can't follow of what a function runs, a `dyn Fn` it's given, is taken to run nothing, so a request that calls one before it returns is quiet, though that may call its callbacks before the cell is set (rd621)",
    file,
    find: "            let Some(reached) = runs.of(id) else { continue };\n",
    replace: "            let reached = runs.of(id).unwrap_or_default();\n",
    tests: corpus("owner_handle_quiet"),
    snapshots: true,
  },
  {
    name: "quiet-unread-body",
    breaks:
      "a function of the crate's whose body rust-js doesn't read, a derived `PartialEq`'s a `dyn` of the crate's trait may run, is taken to run nothing, so the request that calls it is quiet (rd621)",
    file,
    find: "                .all(|body| !names(body) && !unread(body))\n",
    replace: "                .all(|body| !names(body))\n",
    tests: corpus("owner_handle_quiet"),
    snapshots: true,
  },

];
