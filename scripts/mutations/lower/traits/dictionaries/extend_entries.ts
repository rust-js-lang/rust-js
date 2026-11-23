// Mutations of crates/rust-js-lower/src/traits/dictionaries/extend_entries.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "extend-reserve-entry-never",
    breaks: "no std collection's `Extend` dictionary has its own `extend_reserve`, so generic code's `unzip` reserves nothing for a `Vec` side, its capacity 16 where std's `reserve` gives 10 (#2414)",
    file: "crates/rust-js-lower/src/traits/dictionaries/extend_entries.rs",
    find: "        if !writes {\n",
    replace: "        if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^extend_reserve_of_sides_generic_code_fills.rs$"],
  },
  {
    name: "hash-extend-reserve-whole",
    breaks: "a hash set that holds items reserves the whole lower bound it's asked, where hashbrown reserves half, so room for 300 million `u32`s panics on a 32-bit target where std's fits (#2414)",
    file: "crates/rust-js-lower/src/traits/dictionaries/extend_entries.rs",
    find: "            let reserved = Expr::cond(Expr::bin(Op::Eq, size.clone(), Expr::int(0)), asked, half);\n",
    replace: "            let reserved = Expr::cond(Expr::bin(Op::Eq, size.clone(), Expr::int(0)), asked.clone(), asked);\n",
    tests: ["test/corpus.test.ts", "-t", "^hash_set_extend_reserves_half_once_it_holds_items.rs$"],
  },
  {
    name: "hash-extend-reserve-unchecked",
    breaks: "a hash set's `extend_reserve` checks no room, so an endless source reserved into it panics a `Vec`'s `capacity overflow` after it, where hashbrown's `Hash table capacity overflow` comes first (#2414)",
    file: "crates/rust-js-lower/src/traits/dictionaries/extend_entries.rs",
    find: "            self.table_room_for(Expr::bin(Op::Add, size, reserved), entry, span, &mut body);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^extend_reserve_of_an_endless_source_panics_in_order.rs$"],
  },
];
