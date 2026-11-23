// Mutations of crates/rust-js-lower/src/copies/reached.rs (ADR 1191).
import type { Mutation } from "../../../mutations";

const file = "crates/rust-js-lower/src/copies/reached.rs";

export const mutations: Mutation[] = [
  {
    name: "js-facing-result-not-reaching",
    breaks: "an exported function's result doesn't reach JS, so `two_matches` gives JS one players array twice (#2408)",
    file,
    find: "            flow.taint(mir::RETURN_PLACE);",
    replace: "            let _ = mir::RETURN_PLACE;",
    tests: ["test/js-copies.test.ts", "-t", "two matches"],
  },
  {
    name: "param-summary-dropped",
    breaks: "what a crate's function is given doesn't flow into its result, so `start_match`'s players aren't followed back to the clone (#2408)",
    file,
    find: "                    (0..arg_places.len()).map(|at| result && params.contains(&at)).collect()",
    replace: "                    vec![false; arg_places.len()]",
    tests: ["test/js-copies.test.ts", "-t", "two matches"],
  },
  {
    name: "js-function-arguments-not-reaching",
    breaks: "what a callback JS gave is given doesn't reach JS, so it may keep Rust's array (#2408)",
    file,
    find: "                _ if to_js => vec![true; arg_places.len()],",
    replace: "                _ if to_js => vec![false; arg_places.len()],",
    tests: ["test/js-copies.test.ts", "-t", "callback"],
  },
  {
    name: "library-result-carries-nothing",
    breaks: "a library's function's result carries none of its arguments, so `dep::pass(players.clone())` is JS's own array (#2408)",
    file,
    find: "                _ => vec![result; arg_places.len()],",
    replace: "                _ => vec![false; arg_places.len()],",
    tests: ["test/js-copies.test.ts", "-t", "library"],
  },
  {
    name: "mut-store-not-followed",
    breaks: "what's stored through a `&mut` into what JS gets isn't followed, so `push_into(&mut out, players.clone())` shares JS's array (#2408)",
    file,
    find: "            if stores {",
    replace: "            if false && stores {",
    tests: ["test/js-copies.test.ts", "-t", "&mut"],
  },
  {
    name: "clone-parts-not-recorded",
    breaks: "a cloned `Option<Vec<u32>>` records the `Option` alone, so the `Vec` in it, decided part by part, isn't copied (#2408)",
    file,
    find: "        ty::Adt(adt, args) if crate::recognition::is_std_crate(tcx, adt.did()) => args.types().collect(),",
    replace: "        ty::Adt(adt, _) if crate::recognition::is_std_crate(tcx, adt.did()) => Vec::new(),",
    tests: ["test/corpus.test.ts", "-t", "^nullable_let_read_alike.rs$"],
  },
  {
    name: "returned-clone-read-through-parts",
    breaks: "a clone a function returns is taken as read only through its fields, so `first_pair` hands JS the tuple its caller holds (#2408)",
    file,
    find: "    if local == mir::RETURN_PLACE {",
    replace: "    if false && local == mir::RETURN_PLACE {",
    tests: ["test/corpus.test.ts", "-t", "^clone_read_through_its_parts_copies_its_parts\\.rs$"],
  },
  {
    name: "destructured-clone-taken-whole",
    breaks: "a clone read only through its fields, `let (node, depth) = stack[at].clone()`, records its tuple, so the JS reads each part apart where React's destructures (#2408)",
    file,
    find: "                let whole = used_whole(body, destination.local);",
    replace: "                let whole = true || used_whole(body, destination.local);",
    tests: ["test/corpus.test.ts", "-t", "^clone_read_through_its_parts_copies_its_parts\\.rs$"],
  },
];
