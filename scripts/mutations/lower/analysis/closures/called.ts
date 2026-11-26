// Mutations of crates/rust-js-lower/src/analysis/closures/called.rs (#2031, #2050).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "call-by-value-unread",
    breaks: "std's call by value of a type parameter its caller calls by `FnOnce` alone, `Some(1).map(f)` of an `F: FnOnce`, isn't taken as one, so a closure rustc infers `Fn` that holds a value with a destructor, given there, is called by its JS function, which drops nothing (#2031)",
    file: "crates/rust-js-lower/src/analysis/closures/called.rs",
    find: "                    Some(OnceDrop::Body) if given == Some(OnceDrop::Body) || drops_none => By::Value,\n",
    replace: "                    Some(OnceDrop::Body) if drops_none => By::Value,\n",
    tests: ["test/corpus.test.ts", "-t", "^fn_closure_moved_on_to_a_std_once_drops.rs$"],
  },
  {
    name: "call-by-ref-taken-by-value",
    breaks: "std's call by `&` of a closure it's given, `map`'s `FnMut`, is taken as a call by value, so a closure that holds a value with a destructor given to one is refused (#2031)",
    file: "crates/rust-js-lower/src/analysis/closures/called.rs",
    find: "                    _ if given == Some(OnceDrop::Whole) => By::Ref,\n",
    replace: "                    _ if given == Some(OnceDrop::Whole) => By::Value,\n",
    tests: ["test/corpus.test.ts", "-t", "^fn_closure_moved_into_std_map_called_by_ref.rs$"],
  },
  {
    name: "library-callee-taken-as-std",
    breaks: "a library's function the crate has no body of, or its trait's method, is taken as std's, whose calls drop what they call by `&`, so a closure rustc infers `Fn` moved into one whose blanket impl calls it by `FnOnce` keeps its own body, which drops nothing (#2031)",
    file: "crates/rust-js-lower/src/analysis/closures/called.rs",
    find: "        let library = !binding && !fn_trait && !callee.is_local() && !is_std_crate(tcx, callee);\n",
    replace: "        let library = false && !binding && !fn_trait && !callee.is_local() && !is_std_crate(tcx, callee);\n",
    tests: ["test/metamorphic.test.ts", "-t", "fn_closure_moved_through_a_blanket_impl_drops"],
    env: { METAMORPHIC: "all" },
  },
  {
    name: "library-given-made-type-unopened",
    breaks: "a closure moved into a struct, `Once<{closure}>`, given through a type parameter to a library trait's method that may take it apart and call it by value, isn't counted as called either way, so it's neither refused nor dropped (#2609)",
    file: "crates/rust-js-lower/src/analysis/closures/called.rs",
    find: "            closures_in(made, &mut inside);\n",
    replace: "            _ = made;\n",
    tests: ["test/crates-refused.test.ts", "-t", "a closure in a library's struct its impl calls by value is refused"],
  },
  {
    name: "library-lent-made-type-opened",
    breaks: "a closure in a struct lent to a library's generic code, `peek_twice<R: Peek>(r: &R)`, which can't move it out of the reference, is counted as called by value, so it's refused though it's called by `&` alone (#2609)",
    file: "crates/rust-js-lower/src/analysis/closures/called.rs",
    find: "        for &made in on.made.iter().filter(|_| whole) {\n",
    replace: "        for &made in on.made.iter() {\n",
    tests: ["test/crates-pairs-2.test.ts", "-t", "two crates: library_peeks_a_held_closure_by_ref"],
  },
];
