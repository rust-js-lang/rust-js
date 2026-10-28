//! Tidy's passes, in the order they run, in stages, each with what it
//! leaves a body with (ADR 0659): the one list of them, which `tidy` runs
//! and `--tidy-until` stops in. A pass reads what it needs of the body as
//! the body is when it runs, never a fact an earlier pass worked out.

use super::contracts::{self, Contract};
use super::*;

/// One of tidy's rewrites: what it does to a body, by what `hints` say.
pub(super) struct Pass {
    pub(super) name: &'static str,
    pub(super) run: fn(&mut Vec<Stmt>, &Hints),
}

/// A run of passes for one job, and what holds of a body once it's done,
/// checked in a debug build, past what holds of every body
/// (`contracts::ALWAYS`).
pub(super) struct Stage {
    pub(super) name: &'static str,
    pub(super) passes: &'static [Pass],
    pub(super) ensures: &'static [Contract],
    /// The stages run again once it's done, where its passes find they
    /// must (`Hints::again`), each by `tidy` as any stage is.
    pub(super) again: &'static [&'static str],
}

const fn pass(name: &'static str, run: fn(&mut Vec<Stmt>, &Hints)) -> Pass {
    Pass { name, run }
}

/// The stages run again once each run of statements a value's drop ends
/// is one `try { .. } finally { .. }` (ADR 0705): what each statement's own
/// `try` held apart, `let tmp; try { tmp = f(); } catch ..`, read in place.
const UNWOUND: &[&str] = &["names", "in place", "chains"];

/// Each stage, in order.
pub(super) const STAGES: &[Stage] = &[
    Stage {
        name: "flags",
        passes: &[
            // Where a drop's flag is known, no test of it, nor a `try` for it.
            pass("settled flags", |b, h| flags::settled_flags(b, h.flags)),
        ],
        ensures: &[contracts::NOTHING_AFTER_A_JUMP],
        again: &[],
    },
    // Twice: what a block's statements, in its place, are followed by, and
    // a loop's label nothing names then.
    Stage {
        name: "jumps",
        passes: JUMPS,
        ensures: &[contracts::LABELS_NAMED],
        again: &[],
    },
    Stage {
        name: "jumps again",
        passes: JUMPS,
        ensures: &[contracts::LABELS_NAMED],
        again: &[],
    },
    Stage {
        name: "values",
        passes: &[
            // What a label's block, gone, left past a jump.
            pass("unreachable", |b, _| js::statement_lists(b, &mut unreachable_after)),
            pass("or tests", |b, _| bottom_up(b, &mut or_tests)),
            pass("branch values", branch_values_of),
            pass("unread result", |b, h| unread_result(b, h.result)),
            pass("guarded", |b, _| js::statement_lists(b, &mut guarded)),
            pass("declarations", |b, _| declarations(b)),
            pass("aliases", |b, h| {
                aliases(b, h.captured, (&h.params.borrow(), h.kept), h.clones)
            }),
            // A test an earlier one decided, `if (x != null) { return x; } ..
            // if (x != null)`, where nothing between changes what it reads:
            // its branch; and a tail MIR wrote into both branches, once,
            // after them.
            pass("decided tests", decided_tests_of),
            // A flag whose only test was such a drop's, read nowhere now,
            // goes with its writes.
            pass("settled flags", |b, h| flags::settled_flags(b, h.flags)),
            pass("shared tails", |b, h| {
                js::statement_lists(b, &mut |s| shared_tails(s, &h.written.let_elses, &h.written.switches))
            }),
        ],
        ensures: &[contracts::NOTHING_AFTER_A_JUMP, contracts::NO_EMPTY_IF],
        again: &[],
    },
    Stage {
        name: "loops",
        passes: &[
            // The innermost first: what a loop ends with may be another,
            // recovered. An iterator a loop inside another makes, a `let`
            // assigned each time, read by nothing else.
            pass("loops", |b, h| {
                let mut taken = mentions(b);
                bottom_up(b, &mut |stmts| loops(stmts, &mut taken, h.numbers));
                loop_bound_lets(b);
            }),
            // A match's tree of tests, its arms reached through a block MIR
            // made: the chain of its arms, now that the loops are made.
            pass("decisions", |b, _| bottom_up(b, &mut decisions::decisions)),
            // What the program wrote after a loop, MIR moved into its way out.
            pass("exits after", |b, _| js::statement_lists(b, &mut exits_after)),
            pass("early returns", |b, h| {
                js::statement_lists(b, &mut |s| early_returns(s, &h.written.elses))
            }),
            pass("early breaks", |b, _| {
                js::statement_lists(b, &mut |s| early_breaks(s, false))
            }),
            pass("settle jumps", |b, _| settle_jumps(b)),
            // A `let` declared first, its one write now beside it, what a
            // loop held: a `const` there.
            pass("declarations", |b, _| declarations(b)),
            pass("continued", |b, h| {
                js::statement_lists(b, &mut |s| continued(s, &h.written.continues))
            }),
            pass("tail jumps", |b, _| js::statement_lists(b, &mut |s| tail_jumps(s))),
            pass("innermost jumps", |b, _| {
                js::statement_lists(b, &mut |s| innermost_jumps(s))
            }),
            // A chain of tests a loop ended, its body now ending where the
            // chain's does.
            pass("branch values", branch_values_of),
            pass("unlabeled", |b, _| js::statement_lists(b, &mut unlabeled)),
        ],
        ensures: &[contracts::LABELS_NAMED, contracts::NO_EMPTY_IF],
        again: &[],
    },
    Stage {
        name: "logicals",
        passes: &[
            pass("logical blocks", |b, h| {
                let (logicals, returns, booleans) = (&h.written.logicals, h.returns, h.booleans);
                js::statement_lists(b, &mut |s| logical_blocks(s, logicals, returns, booleans))
            }),
            pass("logical returns", |b, h| {
                let (logicals, returns, booleans) = (&h.written.logicals, h.returns, h.booleans);
                js::statement_lists(b, &mut |s| logical_returns(s, logicals, returns, booleans))
            }),
            // A `return a && b && c;` the program wrote, whose tests' failing
            // `return` is after the list (ADR 0947).
            pass("returned logicals", |b, h| {
                returned_logicals(b, &h.written.logicals, h.returns, h.booleans)
            }),
            pass("falsy values", |b, h| falsy_values(b, h.falsy, h.tested)),
            pass("filtered guards", |b, _| {
                let changing = changing(b);
                js::statement_lists(b, &mut |s| filtered_guards(s, &changing))
            }),
        ],
        ensures: &[contracts::LABELS_NAMED, contracts::NO_EMPTY_IF],
        again: &[],
    },
    Stage {
        name: "names",
        passes: &[
            pass("alias variables", |b, _| alias_variables(b)),
            pass("read handles", |b, _| read_handles(b)),
            pass("plain reads", |b, h| {
                js::statement_lists(b, &mut |s| plain_reads(s, h.plain, h.captured))
            }),
            pass("destructured", |b, h| {
                js::statement_lists(b, &mut |s| {
                    destructured(s, (&h.lets.patterns, &h.lets.named, &h.named.user))
                })
            }),
            pass("rest parts", |b, _| js::statement_lists(b, &mut rest_parts)),
            // What an array made for its parts left, `const _5 = 6;`, too.
            pass("parts' aliases", |b, _| alias_variables(b)),
            pass("named temporaries", |b, _| named_temporaries(b)),
            pass("consumed into iterators", |b, h| {
                iterators::consumed_into_iters(b, h.fuse)
            }),
            pass("range items", |b, _| {
                js::each_expr_mut(b, &mut crate::lower::std_types::range::range_items)
            }),
            pass("range steps", |b, _| {
                js::each_expr_mut(b, &mut crate::lower::std_types::range::range_steps)
            }),
            pass("eager chains", |b, _| eager_chains(b)),
            pass("eager spreads", |b, _| {
                js::statement_lists(b, &mut |s| eager_spreads(s))
            }),
            // A block, or branches, a function's body or a loop's ends in,
            // the function's value made in them, `?`'s.
            pass("returned in branches", |b, h| {
                js::statement_lists(b, &mut |s| returned_in_branches(s, h.result))
            }),
            pass("shared tails", |b, h| {
                js::statement_lists(b, &mut |s| shared_tails(s, &h.written.let_elses, &h.written.switches))
            }),
            // Each `let` where it's first named, a `const` where it's
            // written once.
            pass("sunk lets", |b, _| js::statement_lists(b, &mut sunk_lets)),
            pass("declarations", |b, _| declarations(b)),
            // A temporary of a variable those made a `const`, `const _42 =
            // queue;` of a `borrow_mut()` of an object's `RefCell`, which is
            // the object: the variable, where it's read (ADR 0883).
            pass("declared aliases", |b, _| alias_variables(b)),
            // What a `let` declared first holds now taken apart, a guard's
            // `try` too: a temporary's parts, `const [a, b] = f();`. Not a
            // pattern's one part of what the program names, `const front =
            // d[0];`, which the first pass left as it is (ADR 0705).
            pass("destructured declared", |b, h| {
                js::statement_lists(b, &mut |s| {
                    destructured(s, (&HashSet::new(), &h.lets.named, &h.named.user))
                })
            }),
            pass("assigned parts", |b, _| js::statement_lists(b, &mut assigned_parts)),
            // A pattern's copy of a variable, `const error = raw;`: the variable.
            pass("copied bindings", |b, h| {
                js::statement_lists(b, &mut |s| copied_bindings(s, &h.named.user))
            }),
            // A variable given another type, `const lazy = children;`: the
            // variable, where it's read no more (ADR 0598, amended).
            pass("cast bindings", |b, h| {
                js::statement_lists(b, &mut |s| cast_bindings(s, &h.lets.casts))
            }),
            // A tuple read item by item, `children[0]`: its items.
            pass("tuple reads", |b, h| {
                let changing = changing(b);
                let set_elsewhere = set_elsewhere(b);
                js::statement_lists(b, &mut |s| tuple_reads(s, &changing, &set_elsewhere, h.captured))
            }),
            // `dyn` values given to JS, `[{ value, impl }].map((item) =>
            // item.value)`: the values.
            pass("unwrapped values", |b, _| {
                js::statement_lists(b, &mut |s| unwrapped_values(s))
            }),
            // What its passes emptied, an `if` of nothing, gone, its test kept
            // where it does anything, as it ensures (#247).
            pass("never taken", |b, _| bottom_up(b, &mut never_taken)),
        ],
        ensures: &[contracts::LABELS_NAMED, contracts::NO_EMPTY_IF],
        again: &[],
    },
    Stage {
        name: "in place",
        passes: &[
            // A match's table read, `TITLES[kind]`, where it's written,
            // before what it's given to reads it in place.
            pass("table reads", |b, h| {
                js::statement_lists(b, &mut |s| table_reads(s, &h.written.tables))
            }),
            // What an element MIR made only where a test holds reads, as one
            // value, read in place below, where the test holds.
            pass("guarded lets", |b, h| {
                js::statement_lists(b, &mut |s| guarded_lets(s, h.captured))
            }),
            // A chain, each step of which the next needs (ADR 0659): `inlined`
            // reads MIR's temporaries in place, which leaves an `if` each of
            // whose branches writes one variable, `let v; if (c) { v = i; }
            // else { v = 99; }`, and a `let` written once; `branch values`
            // makes a `const` of the one, `const v = c ? i : 99;`, and
            // `declarations` of the other; `inlined in values` reads those
            // in place, `const pair = [c ? i : 99, 2]`. Without any one of
            // them, the corpus reads worse (`mir-tidy-late-consts-kept`,
            // `values-chain-branch-values-off`, `values-chain-declarations-off`,
            // `inlined-in-values-off`).
            pass("inlined", inlined_of),
            // An `Option` made where it's tested, `if let Some(o) =
            // kind.object()`: the test it's made by (ADR 0910); before its
            // branches are one value, `o?.$$typeof` of `if (o) { t =
            // o.$$typeof; } else { t = undefined; }`, which it's no test of.
            pass("tested options", |b, _| {
                let changing = changing(b);
                js::statement_lists(b, &mut |s| tested_options(s, &changing))
            }),
            pass("branch values", branch_values_of),
            pass("declarations", |b, _| declarations(b)),
            pass("inlined in values", inlined_of),
            // A tuple's part held, `const _3$0 = v.pop();`, which an arm's
            // binding takes, by that name.
            pass("named temporaries", |b, _| named_temporaries(b)),
            pass("implied tests", |b, _| {
                let changing = changing(b);
                js::each_expr_mut(b, &mut |e| implied_tests(e, &changing))
            }),
            // A chain `rev()` was given in a temporary, read in place now,
            // `$rev(c.filter(p))`: reversed inward, as `rev_items` makes one
            // it sees, of its source stepped from its back (#205).
            pass("reversed inward", |b, _| {
                js::each_expr_mut(b, &mut crate::lower::mir::iter::reversed_in_place)
            }),
            // A chain read in place now, `$lent($iter(xs)).some(f)` of `const
            // _7 = $iter(xs);`: the array's; and a range made there, its
            // numbers made as they're stepped.
            pass("consumed into iterators", |b, h| {
                iterators::consumed_into_iters(b, h.fuse)
            }),
            pass("range items", |b, _| {
                js::each_expr_mut(b, &mut crate::lower::std_types::range::range_items)
            }),
            pass("range steps", |b, _| {
                js::each_expr_mut(b, &mut crate::lower::std_types::range::range_steps)
            }),
            pass("eager chains", |b, _| eager_chains(b)),
            // Of each, a callback that passes what it's given on to Rust's
            // function, `(x, i) => f(x, i)`: `f` itself.
            pass("passed on", |b, h| {
                js::each_expr_mut(b, &mut |e| passed_on(e, h.forwarding))
            }),
            // A closure read in place, called where it's made, of what reads
            // the same: its value of what it's given, `r._0.data`, as a call
            // of one made in place is (ADR 0449).
            pass("applied arrows", |b, _| js::each_expr_mut(b, &mut applied_arrows)),
            // An element's place checked, then set: the place the check gives.
            pass("checked sets", |b, _| js::statement_lists(b, &mut checked_sets)),
            // Two handles compared, each read in place now, of an item and an
            // object: whether the object is the item, `items[i] === r`.
            pass("same objects", |b, _| {
                js::each_expr_mut(b, &mut crate::lower::pointers::same_object_in_place)
            }),
            // A nullable place's `made ?? null` of what's now read in place,
            // a test whose branches are each a value or `undefined`: the
            // test, `null` for `undefined`.
            pass("stored again", |b, _| js::each_expr_mut(b, &mut Expr::store_again)),
            // A copy made of what's spread, read in place, `{ ...a, ...{ ...b
            // } }`: what it's a copy of.
            pass("spread spreads", |b, _| js::each_expr_mut(b, &mut spread_spreads)),
            // An `if` those emptied, a drop scope's test of a temporary
            // whose binding they read in place, run again after `scopes`
            // took its `try` away (#247): gone, its test kept where it
            // does anything.
            pass("never taken", |b, _| bottom_up(b, &mut never_taken)),
        ],
        ensures: &[contracts::LABELS_NAMED, contracts::NO_EMPTY_IF],
        again: &[],
    },
    Stage {
        name: "chains",
        passes: &[
            // Decided tests and shared tails again, of what's read in place now.
            pass("decided tests", decided_tests_of),
            pass("shared tails", |b, h| {
                js::statement_lists(b, &mut |s| shared_tails(s, &h.written.let_elses, &h.written.switches))
            }),
            // A chain's tests, where each is read in place now: `if (a) { if
            // (f(a)) { .. } }`, `if (a && f(a))`.
            pass("and tests", |b, h| {
                bottom_up(b, &mut |s| and_tests(s, &h.written.conditions))
            }),
            // A `let` an `if let` takes apart, of a place's read only the
            // test after it reads, one test now: read there (ADR 0866).
            pass("tested reads", |b, h| {
                js::statement_lists(b, &mut |s| tested_reads(s, &h.lets.taken_apart))
            }),
            // An `||` the program wrote, its branch of a jump as written
            // (ADR 0790).
            pass("or blocks", |b, h| {
                js::statement_lists(b, &mut |s| logicals::or_blocks(s, &h.written.logicals))
            }),
            // An `if` whose condition mixes `&&`, `||` and `!`, of `if`s that
            // leave a block MIR made: its one test (ADR 0946).
            pass("skipped tests", |b, h| {
                js::statement_lists(b, &mut |s| logicals::skipped_tests(s, &h.written.logicals))
            }),
            // An `if` the program wrote with an `else`, as it wrote it, its
            // tests one now; and the `continue`s its branches end with, which
            // it didn't write, gone where the loop's body ends anyway.
            pass("written elses", |b, h| {
                js::statement_lists(b, &mut |s| written_elses(s, &h.written.elses))
            }),
            // And one it wrote without an `else`, its body in it (ADR 0879,
            // amended).
            pass("written thens", |b, h| {
                js::statement_lists(b, &mut |s| written_thens(s, &h.written.thens))
            }),
            // A check MIR wrote first in a chain's last `else`: the chain's arm.
            pass("chained checks", |b, _| {
                js::statement_lists(b, &mut |s| chained_checks(s))
            }),
            pass("unlabeled", |b, _| js::statement_lists(b, &mut unlabeled)),
            // A flag a block MIR made gives, of a test one now, `bb: { if (a
            // && b) { x = true; break bb; } x = false; }`: `x = a && b`, not
            // a default given first then set.
            pass("flag values", |b, h| {
                js::statement_lists(b, &mut |s| flag_values(s, h.booleans))
            }),
            // A value a block MIR made gives where nothing else does: given
            // first, and the block's `break`s that go where it ends anyway,
            // gone.
            pass("default values", |b, _| {
                js::statement_lists(b, &mut |s| decisions::default_values(s))
            }),
            // Once each block's `else` is made of its `break`s, a block's
            // jumps to one after it, that one.
            pass("jumps past blocks", |b, _| {
                js::statement_lists(b, &mut |s| jumps_past_blocks(s))
            }),
            // A block's `break` a loop that ends it ends with, where `exits
            // after` put the loop: the loop's own.
            pass("blocks loops end", |b, _| {
                js::statement_lists(b, &mut |s| blocks_loops_end(s, false))
            }),
            pass("settle jumps", |b, _| settle_jumps(b)),
            // What those leave of each block MIR made: its `break`s
            // `else`s, `if (c) { s; break l; } t`'s `if (c) { s } else { t
            // }`; and a jump after an `if` one branch of which leaves, in the
            // other (ADR 0745).
            pass("branch ends", |b, _| {
                js::statement_lists(b, &mut unreachable_after);
                js::statement_lists(b, &mut |s| branch_ends(s))
            }),
            pass("jumps into branches", |b, _| {
                js::statement_lists(b, &mut jumps_into_branches)
            }),
            // A block's `return` of one value that some of the paths of the
            // `if` before it go on to, and the rest leave past: in each that
            // goes on, once the others' `break`s end them (ADR 0754).
            pass("returns into branches", |b, _| {
                js::statement_lists(b, &mut |s| returns_into_branches(s))
            }),
            // A chain those leave guarded by what each of its tests says,
            // `typeof s === "string"` of `s === "a"`'s: the chain alone (ADR
            // 0755).
            pass("guarded chains", |b, _| js::statement_lists(b, &mut guarded_chains)),
            pass("settle branches", |b, _| settle_jumps(b)),
            pass("declarations", |b, _| declarations(b)),
            // A loop's early `break`s, its tests read in place now, and its test.
            pass("early breaks", |b, _| {
                js::statement_lists(b, &mut |s| early_breaks(s, true))
            }),
            // A loop's test those read in place now, `while (true) { if
            // (!(m.length > n)) { .. } }`'s of `const _7 = m.length;`.
            pass("while tests", |b, h| {
                js::statement_lists(b, &mut |s| while_tests(s, &h.written.loops, &h.written.breaks))
            }),
            // What those leave at the end, a `return;` the program didn't write.
            pass("tail returns", |b, h| tail_returns(b, h.returns)),
            // A loop's early exits again, of what's taken its shape since.
            pass("early returns", |b, h| {
                js::statement_lists(b, &mut |s| early_returns(s, &h.written.elses))
            }),
            // A chain's arms, as statements, in the order written.
            pass("written chains", |b, _| {
                js::statement_lists(b, &mut |s| written_chains(s))
            }),
            // A chain of `js::strict_equals` tests of one subject, before
            // its last two ways are a returned `?:` (ADR 0958).
            pass("strict switches", |b, h| {
                strict_switches::strict_switches(b, &h.written.strict_tests, &h.written.elses)
            }),
            // `const result = e; return result;` of what the function gives:
            // `return e;`.
            pass("returned consts", |b, h| {
                js::statement_lists(b, &mut |s| returned_consts(s, h.result))
            }),
            // An `if`'s value of two small ones, `return t ? a : b`, its
            // temporaries declared where they're read by now.
            pass("returned values", |b, h| {
                let (elses, returns, booleans) = (&h.written.elses, h.returns, h.booleans);
                js::statement_lists(b, &mut |s| returned_values(s, elses, returns, booleans))
            }),
            // An `if` those emptied, an `if let` whose binding is only read,
            // gone, its test kept where it does anything, as it ensures
            // (#247).
            pass("never taken", |b, _| bottom_up(b, &mut never_taken)),
        ],
        ensures: &[
            contracts::NOTHING_AFTER_A_JUMP,
            contracts::LABELS_NAMED,
            contracts::NO_EMPTY_IF,
        ],
        again: &[],
    },
    Stage {
        name: "written",
        passes: &[
            // A cell a pointer's taken to, made where its value's first set.
            pass("first cell values", |b, _| {
                js::statement_lists(b, &mut first_cell_values)
            }),
            // A cell only its `value` is read and set of, by name: its variable.
            pass("plain cells", |b, _| plain_cells(b)),
            pass("named callbacks", |b, h| {
                js::statement_lists(b, &mut |s| named_callbacks(s, h.callbacks))
            }),
            pass("shown children", |b, h| {
                js::each_expr_mut(b, &mut |e| shown_children(e, h.booleans))
            }),
            pass("concise arrows", |b, h| {
                js::each_expr_mut(b, &mut |e| concise_arrows(e, h.concise))
            }),
            pass("counted entries", |b, _| {
                js::statement_lists(b, &mut crate::lower::std_types::map::counted_entries)
            }),
            // An entry's value put in where it's missing, then used, `if
            // (!m.has(k)) { m.set(k, []); } m.get(k).push(v);`.
            pass("inserted entries", |b, _| {
                js::statement_lists(b, &mut crate::lower::std_types::map::inserted_entries)
            }),
            pass("negations", |b, _| js::each_expr_mut(b, &mut negations)),
            pass("pairs' fields", |b, _| js::each_expr_mut(b, &mut pairs_fields)),
            pass("forwarded values", |b, _| js::statement_lists(b, &mut forwarded_values)),
            pass("unread maps", |b, _| unread_maps(b)),
            // An array's `$iter` an adapter alone steps, left lazy, as what
            // its closures do is in Rust's order: the array's own iterator.
            // A closure's, by the function it's in, which sees what takes
            // what it gives, `xs.flatMap((x) => $iter(x).map(f))`'s, first.
            pass("array values", |b, h| {
                if !h.closure {
                    js::each_expr_mut(b, &mut array_values);
                }
            }),
            pass("handles in place", |b, _| handles_in_place(b)),
            // A value a function given it reads more than once, held where
            // it's made: `const _$1 = f(); .. _$1.TAG ..`, not `((r) =>
            // ..)(f())`.
            pass("applied values", |b, h| {
                let set = js::Writes::of(b).by_closures();
                let mut taken = HashSet::new();
                js::names_in(b, &mut taken);
                js::statement_lists(b, &mut |s| applied_values(s, &set, h.plain, &mut taken))
            }),
            // A `let` declared first, in the body or a block of it, that
            // nothing's left to assign or read.
            pass("unused lets", |b, _| {
                let mentions = mentions(b);
                js::statement_lists(b, &mut |s| {
                    s.retain(|stmt| !matches!(&stmt.kind, StmtKind::Let(name, None) if !mentions.contains_key(name)))
                })
            }),
            // A `const` nothing reads, of a value made only to be dropped,
            // `drop(vec![..])`'s: what making it does.
            pass("unread made", |b, _| {
                let mentions = mentions(b);
                js::statement_lists(b, &mut |s| unread_made(s, &mentions))
            }),
            // A `let` that ends a borrow, of a place's read only the
            // statement after it reads, nothing run first: read there, once
            // what `replace` gave, unread, is gone (ADR 0866, amended).
            pass("ended borrows", |b, h| {
                js::statement_lists(b, &mut |s| ended_borrows(s, &h.lets.ends_borrow))
            }),
            pass("while heads", |b, h| {
                js::statement_lists(b, &mut |s| while_heads(s, &h.written.loops))
            }),
            // An `if` the program wrote without an `else` that a loop
            // `while heads` made ends, its body in it (ADR 0879, amended).
            pass("written thens", |b, h| {
                js::statement_lists(b, &mut |s| written_thens(s, &h.written.thens))
            }),
            pass("written order steps", |b, _| {
                js::statement_lists(b, &mut |s| written_order_steps(s))
            }),
            // A chain of `if`s written without an `else` whose jumps MIR joined
            // after it: each its own (ADR 0879, amended).
            pass("written jumps of ifs", |b, h| {
                js::statement_lists(b, &mut |s| written_jumps_of_ifs(s, &h.written.thens))
            }),
            // The `continue`s a loop's body ends with anyway, which the
            // program didn't write, once those have each loop's shape: of
            // what was after a loop inside it, too, which `while heads` put
            // after it, its label with it (ADR 0702, amended).
            pass("tail continues", |b, h| {
                js::statement_lists(b, &mut |s| tail_continues(s, &h.written.continues))
            }),
            pass("let else first", |b, h| {
                // And where a unit function goes on to its end, as to the
                // `return;` it ends with, each `if` the program wrote before
                // it that returns: its guard (ADR 0879, amended).
                let goes_on = !b.last().is_some_and(ends);
                if goes_on {
                    b.push(StmtKind::Return(None).at(js::Span::NONE));
                }
                js::statement_lists(b, &mut |s| let_else_first(s, &h.written.let_elses));
                if goes_on {
                    written_exits(b, &h.written.thens, h.returns);
                    if b.last().is_some_and(|s| matches!(s.kind, StmtKind::Return(None))) {
                        b.pop();
                    }
                }
            }),
            pass("tried first", |b, _| js::statement_lists(b, &mut tried_first)),
            // What its passes emptied, an `if` of nothing, gone, its test kept
            // where it does anything, as it ensures (#247).
            pass("never taken", |b, _| bottom_up(b, &mut never_taken)),
        ],
        ensures: &[
            contracts::NOTHING_AFTER_A_JUMP,
            contracts::LABELS_NAMED,
            contracts::NO_EMPTY_IF,
        ],
        again: &[],
    },
    Stage {
        name: "settled",
        passes: &[
            // A loop's jumps to the end of the loop it ends, once what's after
            // each loop is placed by them: its own, where the program names
            // it no label; and the early exits those `break`s now make.
            pass("outer continues", |b, h| {
                js::statement_lists(b, &mut |s| outer_continues(s, &h.written.loops))
            }),
            pass("early breaks", |b, _| {
                js::statement_lists(b, &mut |s| early_breaks(s, false))
            }),
            // Their jumps then, of the loop they're now in, unnamed.
            pass("innermost jumps", |b, _| {
                js::statement_lists(b, &mut |s| innermost_jumps(s))
            }),
            // Each `break` a loop that ends a block MIR made leaves it by,
            // once what's after each loop is placed: the loop's own, a
            // `loop` that ends an `if`'s branch, its `else` after it, as
            // written, with no label (ADR 0859); one a loop inside it
            // leaves, its `break 'rows`, the loop's, by the program's name.
            pass("loops ending blocks", |b, h| {
                js::statement_lists(b, &mut |s| blocks_loops_end(s, true));
                js::statement_lists(b, &mut |s| named_labels(s, &h.written.loops))
            }),
            pass("settle jumps", |b, _| settle_jumps(b)),
            // A `while` those leave its own `break`, `while (true) { if
            // (!(t)) { break; } .. }` of one that ends an `if`'s branch with
            // code after the `if`, whose `break` left the block: its test.
            pass("tests of loops ending blocks", |b, h| {
                js::statement_lists(b, &mut |s| while_tests(s, &h.written.loops, &h.written.breaks))
            }),
            // A test the program wrote `!(a && b)`, MIR's `!a || !b`: as
            // written (ADR 0879, amended).
            pass("written negations", |b, h| {
                logicals::written_negations(b, &h.written.negations, &h.written.logicals)
            }),
            // A `!` of a chain the program didn't write, `!matches!(s,
            // Some("a" | "b"))`'s: `s !== "a" && s !== "b"` (ADR 0879, amended).
            pass("unwritten negations", |b, h| {
                logicals::unwritten_negations(b, &h.written.negations, &h.written.logicals)
            }),
            // An `if` of `false`, a check that can't fail once what it reads
            // is in place: its `else`, if any; and an `if` of nothing either
            // way: its test, where that does anything.
            // Inner lists first, as an `if` is emptied by what's taken in it.
            pass("never taken", |b, _| bottom_up(b, &mut never_taken)),
            pass("holed lists", |b, _| holed_lists(b)),
            // A `match` of constants, its chain settled: a `switch` (ADR 0629).
            pass("switches", |b, h| {
                switches::switches(b, &h.written.switches, &h.written.loops)
            }),
            // `!(b >= 48)` of what's never `NaN`: `b < 48`; a guard's too,
            // that a case of a `switch` tests negated (ADR 0905).
            pass("ordered floats", |b, h| {
                js::each_expr_mut(b, &mut |e| ordered_floats(e, h.floats))
            }),
            pass("turned", |b, h| js::each_expr_mut(b, &mut |e| turned(e, h.numbers))),
            // A block the program labeled that a `break` in a `switch` or
            // a loop of a case leaves: the label of the `switch` it leaves
            // (ADR 0944).
            pass("labeled switches", |b, h| labeled_switches(b, &h.written.elses)),
            // A block one case alone reaches, after its `switch`: that
            // case's (ADR 0905).
            pass("blocks one case reaches", |b, _| {
                js::statement_lists(b, &mut |s| case_rests::blocks_one_case_reaches(s))
            }),
            // The rest a block a `switch` ends reaches from one case alone:
            // that case's, at the body's end too (ADR 0905, amended).
            pass("rest one case reaches", |b, _| {
                case_rests::rest_one_case_reaches(b, true);
                js::statement_lists(b, &mut |s| case_rests::rest_one_case_reaches(s, false))
            }),
            // A block's `break` in a `switch` it ends with: the case's own.
            pass("breaks past switches", |b, h| {
                js::statement_lists(b, &mut |s| switches::breaks_past_switches(s, &h.written.switches))
            }),
            // A case's `break` to the `return;` after its `switch`, of an
            // arm the program wrote a `return` in: that `return;` (ADR
            // 0905, amended). And each `continue` the stage left where its
            // loop's turn ends anyway, a default's and React's `if (i !== j)
            // { ..; continue; }` that ends a `for`'s body (ADR 0961).
            pass("returns of cases", |b, h| {
                js::statement_lists(b, &mut |s| {
                    case_rests::returns_of_cases(s, &h.written.switches, h.returns)
                });
                js::statement_lists(b, &mut |s| tail_continues(s, &h.written.continues))
            }),
            // A block a case ends with: its `break`s the case's (ADR 0790).
            pass("blocks ending cases", |b, h| {
                js::statement_lists(b, &mut |s| switches::blocks_ending_cases(s, &h.written.elses))
            }),
            // A loop the program labeled, left by its `switch`'s cases: each
            // case's `break` of its label, as written (ADR 0702, amended).
            pass("loops left by cases", |b, h| {
                js::statement_lists(b, &mut |s| case_rests::loops_left_by_cases(s, &h.written.loops))
            }),
            // `(lanes & X) !== 0` of a test, and `lanes !== 0` of an integer's:
            // `lanes & X`, `lanes` (ADR 0896, amended). After `switches`, whose
            // cases are its chain's `day === 0`.
            pass("bit tests", |b, h| {
                logicals::bit_tests(b, h.numbers, &h.written.patterns)
            }),
            pass("source spans", |b, _| js::statement_lists(b, &mut |s| source_spans(s))),
            // A `continue` or a loop's `break` the program wrote, where it
            // wrote it, which its comment is before (ADR 0684, amended).
            pass("written jumps", |b, h| {
                written_breaks_of_last_loops(b, &h.written.breaks);
                js::statement_lists(b, &mut |s| written_jumps(s, &h.written.continues, &h.written.breaks))
            }),
        ],
        ensures: &[
            contracts::NOTHING_AFTER_A_JUMP,
            contracts::LABELS_NAMED,
            contracts::NO_EMPTY_IF,
            contracts::NO_CONSTANT_TEST,
        ],
        again: &[],
    },
    // The body as it's written, its parameters taken apart, and each run of
    // statements a value's drop ends one `try { .. } finally { .. }` (ADR
    // 0437), a `return` through its `finally`. Where it made any, what each
    // statement's own `try` held apart, `let tmp; try { tmp = f(); } catch
    // ..`, is read as tidy reads any body: its stages that read temporaries
    // in place run again (`UNWOUND`, ADR 0705).
    Stage {
        name: "scopes",
        passes: &[
            pass("parameters", |b, h| destructured_params(&mut h.params.borrow_mut(), b)),
            pass("drop scopes", |b, h| {
                let mut scoped = false;
                js::statement_lists(b, &mut |stmts| {
                    scoped |= unwinds::scoped_drops(stmts, h.result, &h.lets.guards)
                });
                h.again.set(scoped);
            }),
            pass("returns through finally", |b, _| {
                js::statement_lists(b, &mut unwinds::returned_through_finally)
            }),
            // What's left of a call that can't throw, a `catch_unwind`'s,
            // held apart for the drops a scope's `finally` didn't take: as it
            // runs (ADR 0873).
            pass("unthrown catches", |b, h| {
                let mut unthrown = false;
                js::statement_lists(b, &mut |stmts| unthrown |= unwinds::unthrown_catches(stmts));
                h.again.set(h.again.get() || unthrown);
            }),
            // And an `unwrap()`'s, its test and panic, as an `assert!`'s:
            // where its drops' flags are known again (ADR 0873, amended).
            pass("tested unwraps", |b, h| {
                let mut tested = false;
                js::statement_lists(b, &mut |stmts| tested |= unwinds::tested_unwraps(stmts));
                if tested {
                    flags::settled_flags(b, h.flags);
                }
                h.again.set(h.again.get() || tested);
            }),
        ],
        ensures: &[
            contracts::NOTHING_AFTER_A_JUMP,
            contracts::LABELS_NAMED,
            contracts::NO_EMPTY_IF,
            contracts::NO_CONSTANT_TEST,
        ],
        again: UNWOUND,
    },
    // Then its temporaries named, its drop flags too; a shadow of a binding
    // the program names read no more, that binding (ADR 0650); and a
    // `String` appended to twice, once (ADR 0652).
    Stage {
        name: "finish",
        passes: &[
            pass("temporaries named", |b, h| {
                bindings::readable_temporaries(&h.params.borrow(), b, h.suggested)
            }),
            pass("flags named", |b, h| flags::named_flags(b, h.flags)),
            pass("shadows", |b, h| {
                shadows::shadowed(&h.params.borrow(), b, &h.named.user)
            }),
            // A binding of a variable's name the variable's given last, the
            // variable given it where it's made; then a variable given what's
            // read before one way, given it first (ADR 0853).
            pass("given last", |b, h| bindings::given_last(&h.params.borrow(), b)),
            pass("filled lazily", |b, _| {
                js::statement_lists(b, &mut bindings::filled_lazily)
            }),
            pass("appends", |b, h| {
                crate::lower::std_types::text::appended(b, &h.named.text)
            }),
            // A closure that passes on what it captured, then what it's
            // given, to a function of the crate's: that function bound,
            // `g.bind(null, a)`, once nothing calls it in place (ADR 0605,
            // amended).
            pass("bound captures", |b, h| {
                js::each_expr_mut(b, &mut |e| {
                    if let Some(bound) = crate::lower::passed_on::bound(e, h.forwarding) {
                        *e = bound;
                    }
                })
            }),
            // A nullable field's value a variable holds, stored in another
            // nullable place as it is, no `?? null` (ADR 0620, amended).
            pass("nulled as they are", |b, _| values::nulled_as_they_are(b)),
        ],
        // What `settled` ensured, kept: what's printed is checked as it is.
        ensures: &[
            contracts::NOTHING_AFTER_A_JUMP,
            contracts::LABELS_NAMED,
            contracts::NO_EMPTY_IF,
            contracts::NO_CONSTANT_TEST,
        ],
        again: &[],
    },
];

/// `jumps`, run twice.
const JUMPS: &[Pass] = &[
    // A loop that ends the function, whose way out MIR wrote as its
    // `return;`: its `break`, as a loop followed by code is left.
    pass("loop ends returned", |b, _| loop_ends_returned(b)),
    // A block the program labeled, by its name, its `break`s where they're
    // written (ADR 0790).
    pass("written blocks", |b, h| written_blocks(b, &h.written.blocks)),
    // `?` of an `Ok` made here: the branch it takes, before what's past a
    // jump goes.
    pass("made ok tried", |b, _| js::statement_lists(b, &mut made_ok_tried)),
    pass("returned", |b, h| {
        js::statement_lists(b, &mut |s| {
            returned(s, h.result);
            unreachable_after(s);
        })
    }),
    // A block MIR made whose end is a `return;`, which a loop inside it
    // leaves by: each such `break` that `return;`, an early return as
    // written (ADR 0742, amended).
    pass("returns past blocks", |b, h| {
        js::statement_lists(b, &mut |s| returns_past_blocks(s, &h.written.blocks, h.returns))
    }),
    // A `return` of an `if` the program wrote, `return if c { a } else { b
    // };`: its value, `return c ? a : b;`, before a shared tail takes its
    // `else`'s out (ADR 0657, amended).
    pass("returned ifs", |b, h| {
        let (elses, returns, booleans) = (&h.written.elses, h.returns, h.booleans);
        js::statement_lists(b, &mut |s| returned_values(s, elses, returns, booleans))
    }),
    // An arm MIR made the block's fall-through, written first: first.
    pass("written first", |b, h| {
        js::statement_lists(b, &mut |s| written_first(s, &h.written.switches))
    }),
    // A `continue` of a loop around the program wrote, MIR's jump past a
    // block to it: where it wrote it (ADR 0702, amended).
    pass("continues written", |b, h| {
        js::statement_lists(b, &mut |s| continues_written(s, &h.written.continues))
    }),
    pass("branch ends", |b, _| js::statement_lists(b, &mut |s| branch_ends(s))),
    pass("tail jumps", |b, _| js::statement_lists(b, &mut |s| tail_jumps(s))),
    pass("branch values", branch_values_of),
    // A chain's arms that share one jump after it, the rest of their
    // block falling to its end: each arm's own, as written.
    pass("breaks as ends", |b, h| {
        js::statement_lists(b, &mut |s| breaks_as_ends(s, &h.written.blocks))
    }),
    pass("unlabeled", |b, _| js::statement_lists(b, &mut unlabeled)),
    // A let chain's value, each branch one: its binding read first, `const
    // x = a ? e : undefined;`, as an `if let`'s, which the next run and
    // `values` make one value of (ADR 0870).
    pass("chain values", |b, h| {
        js::statement_lists(b, &mut |s| chain_values(s, h.returns))
    }),
];

/// What branches make a value of, the function's too; and the arms of a
/// `match` of constants, a `switch` (ADR 0629), which make none of what the
/// program names.
fn branch_values_of(body: &mut Vec<Stmt>, hints: &Hints) {
    let made = (hints.result, hints.written.switches.as_slice());
    bottom_up(body, &mut |s| {
        branch_values(s, hints.returns, hints.booleans, made, hints.written)
    });
}

fn decided_tests_of(body: &mut Vec<Stmt>, hints: &Hints) {
    let changing = changing(body);
    js::statement_lists(body, &mut |s| decided_tests(s, &changing, &hints.written.ifs));
    // What a branch taken in place of its `if` leaves after its jump,
    // never reached (#270).
    js::statement_lists(body, &mut unreachable_after);
}

/// What's read once, in place, of what the body sets as it is now.
fn inlined_of(body: &mut Vec<Stmt>, hints: &Hints) {
    let changed = changing(body);
    let set = js::Writes::of(body).by_closures();
    let paired = paired_temporaries(body);
    let facts = MoveFacts {
        changing: &changed,
        set: &set,
        plain: hints.plain,
        captured: hints.captured,
        paired: &paired,
    };
    js::statement_lists(body, &mut |s| inlined(s, &facts));
}

/// How many times each name is mentioned.
fn mentions(body: &[Stmt]) -> HashMap<String, usize> {
    let mut mentions: HashMap<String, usize> = HashMap::new();
    js::visit_stmts(body, &mut |name| *mentions.entry(name.to_string()).or_default() += 1);
    mentions
}

#[cfg(test)]
mod tests {
    /// Each pass's name, `stage/pass`, is its own, as `--tidy-until` names it.
    #[test]
    fn each_pass_is_named_apart() {
        let names = super::super::pass_names();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(unique.len(), names.len());
    }
}
