//! What a body's integers can be where it reads them (ADR 0576): each one's
//! range, and its bound by a slice's length, found by walking the body as
//! it runs, each condition narrowing them where it holds. Of these: which
//! index of a `Vec` or a slice is in bounds, read as JS reads it, `heap[i]`
//! (ADR 0292); and which `+`, `-` or `*` of a small integer can't leave
//! its type's range, written without its wrap, `(i + 1) * 2 - 1`. And
//! which `OnceCell` the body made is still empty where it's set (ADR
//! 0847), `payload._ioInfo = ioInfo`.

use super::*;
use crate::lower::bindings::reads_string_length;
use crate::lower::recognition::{
    VecView, counts_utf16_units, is_iterator_enumerate, is_iterator_rev, is_slice_items, is_vec_from_elem, once_method,
    vec_view,
};
use crate::lower::std_types::once::OnceOp;
use rustc_middle::middle::region;
use rustc_middle::ty::TypeVisitableExt;

mod cells;
mod empty;
mod known;
mod narrowing;
mod present;
mod reads;

use cells::reads_a_cell;
use known::{Below, Known, Range, VALUE, by_value, either};
use reads::{calls, pure, read_vars};

/// A `Vec` or a slice, by the variable it's in and the fields to it.
type Slice = (Vec<usize>, LocalVarId);

/// The most a JS string's `length` is: `2^31 - 1`, as every engine's
/// string is at most, V8's below 2^29, SpiderMonkey's 2^30 - 2,
/// JavaScriptCore's 2^31 - 1, so two add without passing `u32`'s most
/// (ADRs 0632, 0682).
const STRING_LENGTH_MOST: i128 = (1 << 31) - 1;

/// The most a slice's length is of items that take up room: `isize`'s
/// most, as no Rust allocation is larger in bytes (ADR 0701). Of items of
/// no size, or of a size not known, `u32`'s most.
const SIZED_SLICE_LENGTH_MOST: i128 = i32::MAX as i128;

/// Where a `break` or a `continue` of a scope goes, and what's known where
/// each is.
struct Target {
    scope: region::Scope,
    breaks: Option<Known>,
    continues: Option<Known>,
}

/// What a body's integer ranges show: each index known in bounds, its
/// `a[i]` or its `Index::index`'s function, and where each `+`, `-` and `*`
/// known to stay in its type's range is written, and each `as` of an
/// integer already in its target's range, its value as it is.
#[derive(Default)]
pub(in crate::lower) struct Ranges {
    pub(in crate::lower) in_bounds: HashSet<ExprId>,
    pub(in crate::lower) exact: HashSet<Span>,
    pub(in crate::lower) kept_casts: HashSet<ExprId>,
    /// The function of each `OnceCell::set` of a cell still empty there.
    pub(in crate::lower) set_empty: HashSet<ExprId>,
}

/// A parameter of a function only calls reach, a variable its pattern
/// binds as it is, and whether it's an integer of a range rust-js knows.
#[derive(Clone, Copy)]
struct Param {
    var: LocalVarId,
    integer: bool,
}

struct Walk<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    thir: &'a Thir<'tcx>,
    /// Each site's judgement: in bounds, exact, or a cast that keeps its
    /// value, each time it's reached.
    indexes: HashMap<ExprId, bool>,
    ops: HashMap<ExprId, bool>,
    casts: HashMap<ExprId, bool>,
    /// Each `OnceCell::set`'s, by its function: its cell empty there.
    sets: HashMap<ExprId, bool>,
    /// The reads of variables that give nothing their cells
    /// (`empty::quiet_reads`).
    quiet: HashSet<ExprId>,
    typing_env: ty::TypingEnv<'tcx>,
    /// The most each slice's length is whose `len()` the body asks, by
    /// what its items are (`SIZED_SLICE_LENGTH_MOST`).
    length_most: HashMap<Slice, i128>,
    /// Variables changed where the walk doesn't see it, by a raw pointer:
    /// never known.
    volatile: HashSet<LocalVarId>,
    changes: HashMap<LocalVarId, Vec<Span>>,
    /// The most each integer variable's type holds.
    most: HashMap<LocalVarId, i128>,
    targets: Vec<Target>,
    /// A jump the walk can't follow: nothing it found holds.
    lost: bool,
    /// The functions only calls reach, by their parameters, and what each
    /// is told of them where this body calls it.
    callees: &'a HashMap<DefId, Vec<Option<Param>>>,
    told: HashMap<DefId, Option<Known>>,
    /// The body's root, and each scope's, whether what it drops where it
    /// ends may run code, once a scope's asked (`cells::scope_drops`).
    root: ExprId,
    scope_drops: Option<HashMap<ExprId, bool>>,
    /// The crate's bodies, a closure's that a test calls (`present`).
    bodies: &'a present::Bodies<'a, 'tcx>,
}

/// A body to walk: what it is, its THIR and its root.
pub(in crate::lower) type Walked<'a, 'tcx> = (DefId, &'a Thir<'tcx>, ExprId);

/// The ranges of each body's integers, and what they show (ADR 0576): of
/// a function only calls reach (`only_called`), its parameters as each
/// call gives them (ADR 0577), joined, walked again until what each is
/// told holds.
pub(in crate::lower) fn ranges<'tcx>(
    tcx: TyCtxt<'tcx>,
    bodies: &[Walked<'_, 'tcx>],
    only_called: &HashMap<DefId, usize>,
) -> Vec<Ranges> {
    // A function only calls reach, each of whose calls is in a body here.
    let mut counted: HashMap<DefId, usize> = HashMap::new();
    for &(_, thir, _) in bodies {
        for callee in calls(thir) {
            *counted.entry(callee).or_default() += 1;
        }
    }
    let callees: HashMap<DefId, Vec<Option<Param>>> = (bodies.iter())
        .filter(|(def_id, ..)| only_called.get(def_id).is_some_and(|n| counted.get(def_id) == Some(n)))
        .map(|&(def_id, thir, _)| {
            let params = (thir.params.iter())
                .map(|param| match param.pat.as_deref()?.kind {
                    PatKind::Binding {
                        var,
                        mode: BindingMode(ByRef::No, _),
                        subpattern: None,
                        ..
                    } => Some(Param {
                        var,
                        integer: type_range(param.ty).is_some(),
                    }),
                    _ => None,
                })
                .collect();
            (def_id, params)
        })
        .collect();
    // What each is told, `None` while no call of it is reached; each body's
    // last walk, and what it told.
    let mut entries: HashMap<DefId, Option<Known>> = callees.keys().map(|&f| (f, None)).collect();
    let mut changed: HashMap<DefId, usize> = HashMap::new();
    let mut found: Vec<Ranges> = bodies.iter().map(|_| Ranges::default()).collect();
    let mut told: Vec<HashMap<DefId, Option<Known>>> = bodies.iter().map(|_| HashMap::new()).collect();
    let index: HashMap<DefId, usize> = (bodies.iter().enumerate())
        .map(|(i, (def_id, ..))| (*def_id, i))
        .collect();
    let all: present::Bodies = bodies
        .iter()
        .map(|&(def_id, thir, root)| (def_id, (thir, root)))
        .collect();
    let mut work: Vec<usize> = (0..bodies.len()).rev().collect();
    let mut queued: Vec<bool> = vec![true; bodies.len()];
    while let Some(i) = work.pop() {
        queued[i] = false;
        let (def_id, thir, root) = bodies[i];
        let entry = match entries.get(&def_id) {
            Some(entry) => entry.clone(),
            None => Some(Known::default()),
        };
        let mut walk = Walk {
            tcx,
            thir,
            indexes: HashMap::new(),
            ops: HashMap::new(),
            casts: HashMap::new(),
            sets: HashMap::new(),
            quiet: empty::quiet_reads(tcx, thir),
            typing_env: ty::TypingEnv::post_analysis(tcx, def_id),
            length_most: HashMap::new(),
            volatile: volatile(thir),
            changes: changes(thir),
            most: most(thir),
            targets: Vec::new(),
            lost: false,
            callees: &callees,
            told: HashMap::new(),
            root,
            scope_drops: None,
            bodies: &all,
        };
        let mut known = entry.map(|mut k| {
            k.vars.retain(|var, _| !walk.volatile.contains(var));
            k
        });
        if known.is_some() {
            walk.expr(root, &mut known);
        }
        // A jump not followed may hide a call: each it makes is told nothing.
        let now = match walk.lost {
            true => (calls(thir).filter(|f| callees.contains_key(f)))
                .map(|f| (f, Some(Known::default())))
                .collect(),
            false => std::mem::take(&mut walk.told),
        };
        found[i] = walk.found();
        let before = std::mem::replace(&mut told[i], now);
        // Each function told otherwise than it was: what all its calls tell
        // it, and walked again.
        let affected: HashSet<DefId> = before.keys().chain(told[i].keys()).copied().collect();
        for callee in affected {
            let joined = (told.iter()).fold(None, |all, by| match by.get(&callee) {
                Some(this) => either(all, this.clone()),
                None => all,
            });
            let times = changed.entry(callee).or_default();
            let entry = entries.get_mut(&callee).expect("a callee");
            // Told otherwise again and again, what still changes is
            // forgotten, and past that, all of it.
            let joined = match (*times, joined, &*entry) {
                (64.., _, _) => Some(Known::default()),
                (4.., Some(mut next), Some(was)) => {
                    next.vars.retain(|v, r| was.vars.get(v) == Some(r));
                    next.lengths.retain(|s, l| was.lengths.get(s) == Some(l));
                    Some(next)
                }
                (_, joined, _) => joined,
            };
            if joined != *entry {
                *entry = joined;
                *times += 1;
                if let Some(&at) = index.get(&callee)
                    && !queued[at]
                {
                    queued[at] = true;
                    work.push(at);
                }
            }
        }
    }
    found
}

impl Walk<'_, '_> {
    /// What the walk judged: each site in bounds or exact every time it was
    /// reached; nothing of a walk that lost its way.
    fn found(&self) -> Ranges {
        if self.lost {
            return Ranges::default();
        }
        // One span may be of more than one operator, a macro's: exact where
        // each is.
        // MIR writes the operator an assignment is of, `s = n - 1`'s, where
        // the assignment is.
        let thir = self.thir;
        let assigned: HashMap<ExprId, Span> = (thir.exprs.iter())
            .filter_map(|e| match e.kind {
                ExprKind::Assign { rhs, .. } => Some((strip(thir, rhs), e.span)),
                _ => None,
            })
            .collect();
        let mut exact: HashMap<Span, bool> = HashMap::new();
        for (&id, &ok) in &self.ops {
            for span in [Some(thir[id].span), assigned.get(&id).copied()].into_iter().flatten() {
                *exact.entry(span).or_insert(true) &= ok && !span.from_expansion();
            }
        }
        Ranges {
            in_bounds: (self.indexes.iter())
                .filter(|&(_, &ok)| ok)
                .map(|(&id, _)| id)
                .collect(),
            exact: (exact.into_iter())
                .filter(|&(_, ok)| ok)
                .map(|(span, _)| span)
                .collect(),
            kept_casts: (self.casts.iter()).filter(|&(_, &ok)| ok).map(|(&id, _)| id).collect(),
            set_empty: (self.sets.iter()).filter(|&(_, &ok)| ok).map(|(&id, _)| id).collect(),
        }
    }
}

/// The variables a raw pointer is made of, which it may change where the
/// walk doesn't see: not a `*const` to what a shared reference refers to,
/// `ptr::eq(&node.children[0], skip)`'s, which nothing may change through it
/// but a cell, whose change is a call the walk sees, `borrow_mut()`.
fn volatile(thir: &Thir<'_>) -> HashSet<LocalVarId> {
    (thir.exprs.iter())
        .filter_map(|expr| match expr.kind {
            ExprKind::RawBorrow {
                mutability: ty::Mutability::Not,
                arg,
            } if through_shared(thir, arg) => None,
            ExprKind::RawBorrow { arg, .. } => root_var(thir, arg),
            _ => None,
        })
        .collect()
}

/// Whether a place is reached through a shared reference, `*r` of an `&T`,
/// on the way to its variable.
fn through_shared(thir: &Thir<'_>, mut e: ExprId) -> bool {
    loop {
        e = strip(thir, e);
        match thir[e].kind {
            ExprKind::Deref { arg } if matches!(thir[arg].ty.kind(), ty::Ref(_, _, ty::Mutability::Not)) => {
                return true;
            }
            ExprKind::Field { lhs, .. } | ExprKind::Index { lhs, .. } | ExprKind::Deref { arg: lhs } => e = lhs,
            _ => return false,
        }
    }
}

/// The most each integer variable the body reads holds, by its type.
fn most(thir: &Thir<'_>) -> HashMap<LocalVarId, i128> {
    (thir.exprs.iter())
        .filter_map(|expr| match expr.kind {
            ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } => {
                type_range(expr.ty).map(|(_, hi)| (id, hi))
            }
            _ => None,
        })
        .collect()
}

/// The integer a literal is, `1` of `i + 1`.
fn literal(thir: &Thir<'_>, e: ExprId) -> Option<i128> {
    match thir[strip(thir, e)].kind {
        ExprKind::Literal { lit, neg } => match lit.node {
            rustc_ast::LitKind::Int(n, _) => Some(if neg { -(n.get() as i128) } else { n.get() as i128 }),
            _ => None,
        },
        _ => None,
    }
}

/// The variable `e` reads, of the body or a closure's capture.
fn var_of(thir: &Thir<'_>, e: ExprId) -> Option<LocalVarId> {
    match thir[strip(thir, e)].kind {
        ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } => Some(id),
        _ => None,
    }
}

/// The variable a place is of, `v` of `v.items[0]` or `*v`.
fn root_var(thir: &Thir<'_>, mut e: ExprId) -> Option<LocalVarId> {
    loop {
        e = strip(thir, e);
        match thir[e].kind {
            ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } => return Some(id),
            ExprKind::Field { lhs, .. } | ExprKind::Index { lhs, .. } | ExprKind::Deref { arg: lhs } => e = lhs,
            _ => return None,
        }
    }
}

/// The range of an integer type rust-js writes as a JS number, of 32 bits
/// at most, whose operators wrap: `usize`'s is `u32`'s.
fn type_range(ty: ty::Ty<'_>) -> Option<(i128, i128)> {
    let bits = match ty.kind() {
        ty::Uint(ty::UintTy::U8) | ty::Int(ty::IntTy::I8) => 8,
        ty::Uint(ty::UintTy::U16) | ty::Int(ty::IntTy::I16) => 16,
        ty::Uint(ty::UintTy::U32 | ty::UintTy::Usize) | ty::Int(ty::IntTy::I32 | ty::IntTy::Isize) => 32,
        _ => return None,
    };
    Some(match ty.is_signed() {
        true => (-(1i128 << (bits - 1)), (1i128 << (bits - 1)) - 1),
        false => (0, (1i128 << bits) - 1),
    })
}

/// Whether a call of `def_id` of `generic_args` counts a string's UTF-16
/// units, its JS `length`, `counts_utf16_units`'s, of its `Self`.
fn reads_utf16_length<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId, generic_args: ty::GenericArgsRef<'tcx>) -> bool {
    (generic_args.types().next()).is_some_and(|iter_ty| counts_utf16_units(tcx, def_id, iter_ty))
}

impl<'a, 'tcx> Walk<'a, 'tcx> {
    /// The slice a place is, through references: `heap` of `*heap`.
    fn slice(&self, e: ExprId) -> Option<Slice> {
        self.slice_through(e).map(|(slice, _)| slice)
    }

    /// The base `e`'s value bounds others by, where it's a variable of an
    /// unsigned integer of 32 bits or fewer: `n` of `i < n` (`VALUE`).
    fn value_base(&self, e: ExprId) -> Option<Slice> {
        let var = var_of(self.thir, e)?;
        let (lo, hi) = type_range(self.thir[e].ty)?;
        (lo == 0 && hi <= u32::MAX as i128 && !self.volatile.contains(&var)).then(|| (vec![VALUE], var))
    }

    /// `range`, of `e`, at most `e`'s value where nothing else bounds it.
    fn bounded_by_itself(&self, mut range: Range, e: ExprId) -> Range {
        if range.below.is_none() {
            range.below = self.value_base(e).map(|base| (base, Below::LENGTH));
        }
        range
    }

    /// What a call asks of a slice's length, and of which.
    fn asks(&self, e: ExprId) -> Option<(SliceLength, ExprId)> {
        let thir = self.thir;
        let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, e)].kind else {
            return None;
        };
        let (def_id, generic_args) = fn_def(thir[strip(thir, fun)].ty)?;
        slice_length(self.tcx, def_id, generic_args).map(|asked| (asked, args[0]))
    }

    /// What `init`, a `vec![item; n]`, is as long as: the slice `n` is the
    /// length of, `terraces` of `terraces.len()`, or `n` itself, an integer
    /// variable (`VALUE`) (#362).
    fn made_as_long(&self, init: ExprId) -> Option<Slice> {
        let thir = self.thir;
        let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, init)].kind else {
            return None;
        };
        let (def_id, _) = fn_def(thir[strip(thir, fun)].ty)?;
        let [_, count] = args[..] else { return None };
        if !is_vec_from_elem(self.tcx, def_id) {
            return None;
        }
        match self.asks(count) {
            Some((SliceLength::Len, items)) => self.slice(items),
            _ => self.value_base(count),
        }
    }

    /// What `fun` makes of a `Vec`, as long as it is.
    fn vec_view(&self, fun: ExprId) -> Option<VecView> {
        let (def_id, generic_args) = fn_def(self.thir[strip(self.thir, fun)].ty)?;
        vec_view(self.tcx, def_id, generic_args)
    }

    /// The most the length `fun`, a slice's `len()`, reads is: below
    /// `isize`'s most of items that take up room (ADR 0701), `u32`'s of
    /// the rest.
    fn length_most_of(&self, fun: ExprId) -> i128 {
        let item = fn_def(self.thir[strip(self.thir, fun)].ty).and_then(|(_, args)| args.types().next());
        let sized = item.filter(|item| !item.has_param()).is_some_and(|item| {
            (self
                .tcx
                .layout_of(ty::TypingEnv::fully_monomorphized().as_query_input(item)))
            .is_ok_and(|layout| !layout.is_zst())
        });
        match sized {
            true => SIZED_SLICE_LENGTH_MOST,
            false => u32::MAX as i128,
        }
    }

    /// Of `test`, a `let`'s, a `bool` that reads without changing anything,
    /// `xs.len() > 3`: the variables it reads, none a raw pointer may
    /// change (ADR 0806).
    fn held_test(&self, test: ExprId) -> Option<Vec<LocalVarId>> {
        let thir = self.thir;
        if !thir[test].ty.is_bool() {
            return None;
        }
        // Not one through a `RefCell`, which other code may change while
        // the variable holds it (ADR 0881).
        if reads_a_cell(self.tcx, thir, test) {
            return None;
        }
        read_vars(self.tcx, thir, test).filter(|reads| reads.iter().all(|var| !self.volatile.contains(var)))
    }

    /// What's known after `var` is changed.
    fn forget(&self, var: Option<LocalVarId>, known: &mut Option<Known>) {
        if let (Some(var), Some(k)) = (var, known.as_mut()) {
            k.changed(var);
        }
    }

    /// The range of `e` where `known` is, an integer's, walking it: what it
    /// changes forgotten, where it jumps followed, its sites judged. `None`
    /// of what isn't an integer rust-js knows, or where the body doesn't
    /// go on past it.
    fn expr(&mut self, e: ExprId, known: &mut Option<Known>) -> Option<Range> {
        let thir = self.thir;
        known.as_ref()?;
        let ty_range = type_range(thir[e].ty);
        let top = || ty_range.map(|(lo, hi)| Range::of(lo, hi));
        match thir[e].kind {
            ExprKind::Scope {
                region_scope, value, ..
            } => {
                let value_range = self.scoped(region_scope, top(), known, |walk, known| walk.expr(value, known));
                // What it drops where it ends may change what a `RefCell`
                // holds (ADR 0881).
                if let Some(k) = known.as_mut()
                    && k.knows_cells()
                {
                    let root = self.root;
                    let drops = (self.scope_drops)
                        .get_or_insert_with(|| cells::scope_drops(self.tcx, self.typing_env, thir, root));
                    if drops.get(&e).copied().unwrap_or(true) {
                        k.cells_changed(None);
                    }
                }
                value_range
            }
            ExprKind::Use { source }
            | ExprKind::NeverToAny { source }
            | ExprKind::ValueTypeAscription { source, .. }
            | ExprKind::PlaceTypeAscription { source, .. } => self.expr(source, known),
            ExprKind::Literal { lit, neg } => match lit.node {
                rustc_ast::LitKind::Int(n, _) if ty_range.is_some() => {
                    let n = n.get() as i128;
                    let n = if neg { -n } else { n };
                    Some(Range::of(n, n))
                }
                _ => top(),
            },
            ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } => {
                self.mention(e, id, known);
                (known.as_ref()).and_then(|k| k.vars.get(&id).cloned()).or_else(top)
            }
            ExprKind::Block { block } => self.block(block, known),
            ExprKind::If {
                cond, then, else_opt, ..
            } => {
                self.expr(cond, known);
                let mut yes = known.clone();
                let mut no = known.take();
                self.refine(cond, true, &mut yes);
                self.refine(cond, false, &mut no);
                let a = self.expr(then, &mut yes);
                let b = else_opt.and_then(|other| self.expr(other, &mut no));
                let value = match (&yes, &no) {
                    (Some(_), Some(_)) => a.zip(b).map(|(a, b)| a.join(&b)),
                    (Some(_), None) => a,
                    (None, Some(_)) => b,
                    (None, None) => None,
                };
                *known = either(yes, no);
                value.or_else(top)
            }
            ExprKind::Loop { body } => {
                self.walk_loop(body, None, known);
                None
            }
            ExprKind::Break { label, value } => {
                if let Some(value) = value {
                    self.expr(value, known);
                }
                self.jump(label, true, known);
                None
            }
            ExprKind::Continue { label } => {
                self.jump(label, false, known);
                None
            }
            ExprKind::Return { value } => {
                if let Some(value) = value {
                    self.expr(value, known);
                }
                *known = None;
                None
            }
            ExprKind::Become { value } => {
                self.expr(value, known);
                *known = None;
                None
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let l = self.expr(lhs, known);
                let r = self.expr(rhs, known);
                match (l, r, ty_range) {
                    (Some(l), Some(r), Some(bounds)) => {
                        let of = (self.value_base(lhs), self.value_base(rhs));
                        Some(self.op(e, op, (&l, &r), bounds, of))
                    }
                    _ => top(),
                }
            }
            ExprKind::Assign { lhs, rhs } => {
                let value = self.expr(rhs, known);
                self.assign(lhs, value, known);
                // The value it replaces is dropped, which may change what a
                // `RefCell` holds, but one whose guard is held (ADR 0881).
                if let Some(k) = known.as_mut()
                    && k.knows_cells()
                    && cells::drop_may_change_cells(self.tcx, self.typing_env, thir[lhs].ty)
                {
                    k.cells_changed(self.cell_of(lhs).as_ref());
                }
                None
            }
            ExprKind::AssignOp { op, lhs, rhs } => {
                let current = self.expr(lhs, known);
                let r = self.expr(rhs, known);
                let value = match (current, r, type_range(thir[lhs].ty)) {
                    (Some(l), Some(r), Some(bounds)) => {
                        let of = (self.value_base(lhs), self.value_base(rhs));
                        Some(self.op(e, op.into(), (&l, &r), bounds, of))
                    }
                    _ => None,
                };
                self.assign(lhs, value, known);
                None
            }
            ExprKind::Call { fun, ref args, .. } => self.call(e, fun, args, known).or_else(top),
            ExprKind::Index { lhs, index } => {
                self.expr(lhs, known);
                let at = self.expr(index, known);
                self.judge_index(e, lhs, at, known);
                top()
            }
            ExprKind::Borrow { arg, borrow_kind } => {
                self.place(arg, known);
                // A `&mut` kept: what changes through it isn't seen, a
                // `RefCell`'s value's too (ADR 0881).
                if matches!(borrow_kind, BorrowKind::Mut { .. }) {
                    self.forget(self.changed_place(arg), known);
                }
                None
            }
            ExprKind::RawBorrow { arg, .. } => {
                self.place(arg, known);
                None
            }
            ExprKind::Match {
                scrutinee, ref arms, ..
            } => {
                if let Some(for_loop) = (BodyQuery { tcx: self.tcx, thir }).as_for(e) {
                    self.walk_for(for_loop, known);
                    return None;
                }
                self.expr(scrutinee, known);
                let before = known.take();
                for &arm in arms.iter() {
                    let mut here = before.clone();
                    self.forget_bound(&thir[arm].pattern, &mut here);
                    if let Some(guard) = thir[arm].guard {
                        self.expr(guard, &mut here);
                    }
                    self.expr(thir[arm].body, &mut here);
                    *known = either(known.take(), here);
                }
                top()
            }
            ExprKind::Let { expr, ref pat } => {
                self.expr(expr, known);
                self.forget_bound(pat, known);
                None
            }
            ExprKind::LogicalOp { op, lhs, rhs } => {
                // Its right only where its left lets it run.
                self.expr(lhs, known);
                let mut runs = known.clone();
                self.refine(lhs, matches!(op, LogicalOp::And), &mut runs);
                self.expr(rhs, &mut runs);
                if runs.is_some() {
                    *known = either(known.take(), runs);
                }
                None
            }
            // An integer `as` another whose range it's already in is the
            // same number, `s as usize` of an `s >= 0` (ADR 0701).
            ExprKind::Cast { source } if ty_range.is_some() && type_range(thir[source].ty).is_some() => {
                let value = self.expr(source, known);
                let kept = value.filter(|v| ty_range.is_some_and(|(lo, hi)| lo <= v.lo && v.hi <= hi));
                *self.casts.entry(e).or_insert(true) &= kept.is_some();
                kept.or_else(top)
            }
            ExprKind::Deref { arg }
            | ExprKind::Unary { arg, .. }
            | ExprKind::Cast { source: arg }
            | ExprKind::PointerCoercion { source: arg, .. }
            | ExprKind::Field { lhs: arg, .. }
            | ExprKind::Repeat { value: arg, .. }
            | ExprKind::Yield { value: arg }
            | ExprKind::ByUse { expr: arg, .. }
            | ExprKind::Reborrow { source: arg, .. }
            | ExprKind::PlaceUnwrapUnsafeBinder { source: arg }
            | ExprKind::ValueUnwrapUnsafeBinder { source: arg }
            | ExprKind::WrapUnsafeBinder { source: arg } => {
                self.expr(arg, known);
                top()
            }
            ExprKind::Tuple { ref fields } | ExprKind::Array { ref fields } => {
                for &field in fields.iter() {
                    self.expr(field, known);
                }
                None
            }
            ExprKind::Adt(ref adt) => {
                for field in adt.fields.iter() {
                    self.expr(field.expr, known);
                }
                if let thir::AdtExprBase::Base(ref base) = adt.base {
                    self.expr(base.base, known);
                }
                None
            }
            // What it captures by `&mut` it may change whenever it's called,
            // and, as Rust's borrows have it, nothing else reads meanwhile:
            // forgotten as it's made.
            ExprKind::Closure(ref closure) => {
                for &upvar in closure.upvars.iter() {
                    self.expr(upvar, known);
                }
                None
            }
            // A `const` of an integer is its value, as a literal (ADR 0720).
            ExprKind::NamedConst { def_id, args, .. } if ty_range.is_some() => constant_integer(self.tcx, def_id, args)
                .map(|n| Range::of(n, n))
                .or_else(top),
            ExprKind::NonHirLiteral { .. }
            | ExprKind::ZstLiteral { .. }
            | ExprKind::NamedConst { .. }
            | ExprKind::ConstParam { .. }
            | ExprKind::ConstBlock { .. }
            | ExprKind::StaticRef { .. }
            | ExprKind::ThreadLocalRef(_) => top(),
            // A `#[loop_match]`'s jump, or assembly's: not followed.
            ExprKind::LoopMatch { .. } | ExprKind::ConstContinue { .. } | ExprKind::InlineAsm(_) => {
                self.lost = true;
                *known = None;
                None
            }
        }
    }

    /// A place walked where it's set or borrowed, not read: what its
    /// indexes compute.
    fn place(&mut self, e: ExprId, known: &mut Option<Known>) {
        match var_of(self.thir, e) {
            Some(var) => self.mention(strip(self.thir, e), var, known),
            None => _ = self.expr(e, known),
        }
    }

    /// A read of `var` at `e`: unless it's one that gives nothing its cells
    /// (`empty::quiet_reads`), none it reaches is known empty after.
    fn mention(&self, e: ExprId, var: LocalVarId, known: &mut Option<Known>) {
        if !self.quiet.contains(&e)
            && let Some(k) = known.as_mut()
        {
            k.escaped(var);
        }
    }

    /// A call: its arguments in order, then what it changes. A `&mut` it's
    /// given may change what it's of; a `Vec`'s length too, unless the call
    /// keeps it, or a `push` or a `pop` says by how much. A call of a
    /// function only calls reach tells it what its parameters are.
    fn call(&mut self, e: ExprId, fun: ExprId, args: &[ExprId], known: &mut Option<Known>) -> Option<Range> {
        let thir = self.thir;
        let lent = |arg: ExprId| match thir[strip(thir, arg)].kind {
            ExprKind::Borrow {
                borrow_kind: BorrowKind::Mut { .. },
                arg: place,
            } => Some(place),
            _ => None,
        };
        self.expr(fun, known);
        let mut ranges = Vec::new();
        for &arg in args {
            ranges.push(match lent(arg) {
                Some(place) => {
                    self.place(place, known);
                    None
                }
                None => self.expr(arg, known),
            });
        }
        let before = known.clone();
        self.tell_callee(fun, args, &ranges, &before);
        // A `OnceCell`'s `set`: of a cell still empty, every time it's
        // reached, and set after (ADR 0847). Of another, which may be one
        // of those its variable reaches, none of them is known empty after.
        if let Some((def_id, _)) = fn_def(thir[strip(thir, fun)].ty)
            && once_method(self.tcx, def_id) == Some(OnceOp::Set)
        {
            let cell = args.first().and_then(|&cell| empty::cell_place(self.tcx, thir, cell));
            let empty = match (cell, known.as_mut()) {
                (Some((cell, _)), Some(k)) => {
                    let empty = k.empty.remove(&cell);
                    if !empty {
                        k.escaped(cell.1);
                    }
                    empty
                }
                _ => false,
            };
            *self.sets.entry(fun).or_insert(true) &= empty;
        }
        let asked = self.asks(e);
        let grows = match asked {
            Some((SliceLength::Push, items)) => self.slice(items).map(|slice| (slice, 1)),
            Some((SliceLength::Pop, items)) => self.slice(items).map(|slice| (slice, -1)),
            _ => None,
        };
        let keeps = self.vec_view(fun).is_some() || self.cell_view(fun).is_some() || grows.is_some();
        for &arg in args {
            if let Some(place) = lent(arg)
                && let (Some(var), Some(k)) = (self.changed_place(place), known.as_mut())
            {
                // A slice's `&mut` can't change its length, nor what a test
                // of it reads.
                let slice = matches!(thir[strip(thir, arg)].ty.kind(), ty::Ref(_, inner, _) if inner.is_slice());
                match slice || keeps {
                    true => _ = k.vars.remove(&var),
                    false => k.changed(var),
                }
            }
        }
        if let (Some((slice, by)), Some(k)) = (grows, known.as_mut()) {
            k.grown(&slice, by);
        }
        // Any call but one that only reads or views what a `RefCell` holds
        // may change it, by another reference to it (ADR 0881).
        let views = self.vec_view(fun).is_some()
            || self.cell_view(fun).is_some()
            || matches!(
                asked,
                Some((
                    SliceLength::Len
                        | SliceLength::IsEmpty
                        | SliceLength::Index
                        | SliceLength::Range
                        | SliceLength::Swap,
                    _
                ))
            );
        if !views && let Some(k) = known.as_mut() {
            k.cells_changed(None);
        }
        match asked {
            // Its indexes where it's called, the length as it was.
            Some((SliceLength::Index | SliceLength::Swap | SliceLength::SwapRemove, items)) => {
                for index in ranges.into_iter().skip(1) {
                    self.judge_index(fun, items, index, &before);
                }
                // An index it's given is below the length once it's given
                // back, where it panics otherwise (ADR 0881).
                if matches!(asked, Some((SliceLength::Index, _))) {
                    self.indexed(items, args.get(1).copied(), known);
                }
                None
            }
            Some((SliceLength::Range | SliceLength::Drain, items)) => {
                self.judge_range(fun, items, args[1], &before);
                None
            }
            Some((SliceLength::Len, items)) => self.slice(items).map(|slice| {
                let least = (known.as_ref())
                    .and_then(|k| k.lengths.get(&slice).copied())
                    .unwrap_or(0);
                let most = self.length_most_of(fun);
                self.length_most.insert(slice.clone(), most);
                Range {
                    lo: least,
                    hi: most,
                    below: Some((slice, Below::LENGTH)),
                }
            }),
            // A JS string's `length`, a binding's, or its count of UTF-16
            // units, `s.encode_utf16().count()` (ADR 0632).
            _ => fn_def(thir[strip(thir, fun)].ty)
                .filter(|&(def_id, generic_args)| {
                    reads_string_length(self.tcx, def_id) || reads_utf16_length(self.tcx, def_id, generic_args)
                })
                .map(|_| Range::of(0, STRING_LENGTH_MOST)),
        }
    }

    /// A call of a function only calls reach, `before` it runs: what it's
    /// told of its parameters, each integer's range and its bound by the
    /// slice another is given, and each slice's least length, joined into
    /// what its other calls here told it.
    fn tell_callee(&mut self, fun: ExprId, args: &[ExprId], ranges: &[Option<Range>], before: &Option<Known>) {
        let Some(k) = before else { return };
        let Some((callee, _)) = fn_def(self.thir[strip(self.thir, fun)].ty) else {
            return;
        };
        let Some(params) = self.callees.get(&callee) else {
            return;
        };
        let slices: Vec<Option<Slice>> = (args.iter().zip(params))
            .map(|(&arg, param)| param.filter(|p| !p.integer).and_then(|_| self.slice(arg)))
            .collect();
        // The parameter a slice of this body is given as.
        let theirs = |slice: &Slice| {
            (slices.iter().zip(params))
                .find_map(|(given, param)| param.filter(|_| given.as_ref() == Some(slice)))
                .map(|param| (Vec::new(), param.var))
        };
        let mut told = Known::default();
        for ((param, range), slice) in params.iter().zip(ranges).zip(&slices) {
            let Some(param) = param else { continue };
            if let (true, Some(range)) = (param.integer, range) {
                let below = (range.below.as_ref()).and_then(|(slice, bound)| Some((theirs(slice)?, *bound)));
                told.vars.insert(param.var, Range { below, ..range.clone() });
            }
            if let Some(least) = slice.as_ref().and_then(|slice| k.lengths.get(slice)) {
                told.lengths.insert((Vec::new(), param.var), *least);
            }
        }
        let entry = self.told.entry(callee).or_insert(None);
        *entry = either(entry.take(), Some(told));
    }

    /// A `break` or a `continue` to `label`: what's known here goes there.
    fn jump(&mut self, label: region::Scope, breaks: bool, known: &mut Option<Known>) {
        let here = known.take();
        match self.targets.iter_mut().rev().find(|t| t.scope == label) {
            Some(target) => {
                let to = if breaks {
                    &mut target.breaks
                } else {
                    &mut target.continues
                };
                *to = either(to.take(), here);
            }
            None => self.lost = true,
        }
    }

    /// `lhs = value`: a variable's range, or what it's in forgotten.
    fn assign(&mut self, lhs: ExprId, value: Option<Range>, known: &mut Option<Known>) {
        let thir = self.thir;
        self.place(lhs, known);
        if let Some(var) = var_of(thir, lhs) {
            self.forget(Some(var), known);
            if let (Some(mut value), Some(k)) = (value, known.as_mut())
                && !self.volatile.contains(&var)
            {
                // `i = i + 1`'s bound by the `i` it was is none of the new.
                if value.below.as_ref().is_some_and(|((_, v), _)| *v == var) {
                    value.below = None;
                }
                k.vars.insert(var, value);
            }
            return;
        }
        // An item of a slice set keeps its length.
        if !matches!(thir[strip(thir, lhs)].kind, ExprKind::Index { .. }) {
            self.forget(self.changed_place(lhs), known);
        }
    }

    /// What `walk` finds in `scope`, which a `break` to it leaves: what's
    /// known where each is joined in where it ends, and its value any of
    /// its type's, `top`, where one gives it.
    fn scoped(
        &mut self,
        scope: region::Scope,
        top: Option<Range>,
        known: &mut Option<Known>,
        walk: impl FnOnce(&mut Self, &mut Option<Known>) -> Option<Range>,
    ) -> Option<Range> {
        self.targets.push(Target {
            scope,
            breaks: None,
            continues: None,
        });
        let value = walk(self, known);
        let target = self.targets.pop().expect("pushed");
        match target.breaks {
            Some(broken) => {
                *known = either(known.take(), Some(broken));
                top
            }
            None => value,
        }
    }

    /// A block's statements and value; of a labeled one, `'desc: { .. }`,
    /// a scope its `break 'desc` leaves, the block's own (ADR 0720).
    fn block(&mut self, block: thir::BlockId, known: &mut Option<Known>) -> Option<Range> {
        let block = &self.thir[block];
        let walk = |walk: &mut Self, known: &mut Option<Known>| {
            for &stmt in &block.stmts {
                walk.stmt(stmt, known);
            }
            block.expr.and_then(|value| walk.expr(value, known))
        };
        if !block.targeted_by_break {
            return walk(self, known);
        }
        let top = (block.expr.and_then(|e| type_range(self.thir[e].ty))).map(|(lo, hi)| Range::of(lo, hi));
        self.scoped(block.region_scope, top, known, walk)
    }

    fn stmt(&mut self, stmt: thir::StmtId, known: &mut Option<Known>) {
        match self.thir[stmt].kind {
            thir::StmtKind::Expr { expr, .. } => {
                self.expr(expr, known);
            }
            thir::StmtKind::Let {
                ref pattern,
                initializer,
                else_block,
                ..
            } => {
                let value = initializer.and_then(|init| self.expr(init, known));
                if let Some(otherwise) = else_block {
                    // It never goes on: nothing it learns is known after.
                    let mut gone = known.clone();
                    self.block(otherwise, &mut gone);
                }
                // What it's made of whose cells it then reaches (ADR 0847).
                let made_of: Vec<LocalVarId> = match (initializer, else_block) {
                    (Some(init), None) => (empty::made_of(self.tcx, self.thir, init).into_iter())
                        .map(|(_, var)| var)
                        .collect(),
                    _ => Vec::new(),
                };
                self.forget_bound(pattern, known);
                if let PatKind::Binding {
                    var,
                    ty,
                    mode: BindingMode(ByRef::No, _),
                    subpattern: None,
                    ..
                } = pattern.kind
                    && let Some(k) = known.as_mut()
                {
                    // Of one a raw pointer is taken of too, which its read
                    // there gives what it reaches.
                    self.reach(var, ty, &made_of, initializer, k);
                    if self.volatile.contains(&var) {
                        return;
                    }
                    if let Some(value) = value {
                        k.vars.insert(var, value);
                    }
                    // A table as long as another, `vec![0.0; terraces.len()]` (#362).
                    if let Some(base) = initializer.and_then(|init| self.made_as_long(init)) {
                        k.same.insert((Vec::new(), var), base);
                    }
                    // A test held, `let truncated = xs.len() > 3;` (ADR 0806).
                    if let Some(test) = initializer
                        && let Some(reads) = self.held_test(test)
                    {
                        k.tests.insert(var, (test, reads));
                    }
                }
            }
        }
    }

    /// What `var`, a `let`'s of a `ty` made of `made_of`'s variables,
    /// reaches of the cells the body made: theirs, and those `initializer`
    /// makes, empty (ADR 0847). Of one whose drop may run code, which
    /// could set theirs, those are no longer known empty.
    fn reach(
        &self,
        var: LocalVarId,
        ty: ty::Ty<'tcx>,
        made_of: &[LocalVarId],
        initializer: Option<ExprId>,
        k: &mut Known,
    ) {
        let of: HashSet<LocalVarId> = (made_of.iter())
            .flat_map(|v| k.reaches.get(v).into_iter().flatten().copied())
            .collect();
        if !of.is_empty() {
            match empty::drop_runs_code(self.tcx, self.typing_env, ty) {
                true => made_of.iter().for_each(|&v| k.escaped(v)),
                false => k.reaches.entry(var).or_default().extend(of),
            }
        }
        let made = initializer.map_or_else(Vec::new, |init| empty::made_empty(self.tcx, self.thir, init));
        if !made.is_empty() {
            k.reaches.entry(var).or_default().insert(var);
            k.empty.extend(made.into_iter().map(|path| (path, var)));
        }
    }

    /// Each variable `pattern` binds, unknown.
    fn forget_bound(&self, pattern: &Pat<'tcx>, known: &mut Option<Known>) {
        let Some(k) = known.as_mut() else { return };
        pattern.walk_always(|p| {
            if let PatKind::Binding { var, .. } = p.kind {
                k.changed(var);
            }
        });
    }

    /// A loop of `body`, its scope the innermost: walked from what's known
    /// at its start each time round, the join of what's known as it's
    /// entered and as each turn goes round again, until that holds. Past
    /// it, what's known where it's left: its scope's `break`s, joined in
    /// where the scope ends. `each`, a `for`'s variable, is set as each
    /// turn starts, and its iterator, run out, leaves where a turn starts.
    fn walk_loop(&mut self, body: ExprId, each: Option<(&Pat<'tcx>, Option<Range>)>, known: &mut Option<Known>) {
        let scope = self.targets.len().checked_sub(1);
        let entry = known.take();
        let mut head = entry.clone();
        for round in 0.. {
            let mut here = head.clone();
            if let Some((pat, value)) = &each {
                self.forget_bound(pat, &mut here);
                // Its item, or an `enumerate()`'s index, `(i, x)`'s `i` (#363).
                let index = match &pat.kind {
                    PatKind::Leaf { subpatterns } => (subpatterns.iter())
                        .find(|field| field.field.index() == 0)
                        .map(|field| &field.pattern),
                    _ => Some(*pat),
                };
                if let Some(PatKind::Binding {
                    var,
                    mode: BindingMode(ByRef::No, _),
                    subpattern: None,
                    ..
                }) = index.map(|index| &index.kind)
                    && let (Some(value), Some(k)) = (value, here.as_mut())
                {
                    k.vars.insert(*var, value.clone());
                }
            }
            self.expr(body, &mut here);
            let continued = scope.and_then(|at| self.targets[at].continues.take());
            let mut next = either(entry.clone(), either(here, continued));
            // Past a few rounds, what still changes is forgotten: what's
            // left holds. An integer still growing is taken first as at
            // most one below its type's most, where its `+ 1` can't wrap,
            // a counter's below a length; growing past that, anything
            // (ADR 0632). One found within what it was taken as, a
            // counter's below a string's length, keeps it: it holds
            // (ADR 0682).
            if round >= 3
                && let (Some(next), Some(head)) = (next.as_mut(), &head)
            {
                let most = &self.most;
                let length_most = &self.length_most;
                let vars = std::mem::take(&mut next.vars);
                next.vars = (vars.into_iter())
                    .filter_map(|(v, r)| {
                        let was = head.vars.get(&v)?;
                        if was.join(&r) == *was {
                            return Some((v, was.clone()));
                        }
                        // Still at most what it was by a length, a counter
                        // below `xs.len()`: at most that, where the length
                        // is its most (ADR 0701).
                        let below = match (&was.below, &r.below) {
                            (Some((s, x)), Some((t, y))) if s == t && x.join(*y) == *x => Some((s.clone(), *x)),
                            _ => None,
                        };
                        // One bound by a length is at most what that bound
                        // is, up to its type's most: the length holds it,
                        // not a `+ 1` that can't wrap (ADR 0709).
                        let type_most = *most.get(&v)?;
                        // A bound by another's value is at most what that
                        // one's type holds.
                        let base_most = |s: &Slice| match by_value(s) {
                            true => most.get(&s.1).copied(),
                            false => length_most.get(s).copied(),
                        };
                        let hi = match &below {
                            Some((s, x)) => type_most.min(x.most(base_most(s).unwrap_or(u32::MAX as i128))),
                            None => type_most - 1,
                        };
                        (was.hi < hi && r.hi <= hi && r.lo >= was.lo).then(|| {
                            let widened = Range { lo: was.lo, hi, below };
                            (v, widened)
                        })
                    })
                    .collect();
                next.lengths.retain(|s, l| head.lengths.get(s) == Some(l));
            }
            if next == head {
                break;
            }
            // Each round past widening forgets something; one that never
            // settles is a bug, and nothing found is trusted.
            if round == 64 {
                self.lost = true;
                break;
            }
            head = next;
        }
        *known = match each {
            Some(_) => head,
            None => None,
        };
    }

    /// `for pat in head { body }`. Of a range, `0..xs.len()`, its variable is
    /// in it, and below the slice's length where the body never changes the
    /// slice: the range's end is its length as the loop began.
    fn walk_for(&mut self, for_loop: ForLoop<'a, 'tcx>, known: &mut Option<Known>) {
        let thir = self.thir;
        let mut head = strip(thir, for_loop.head);
        // A range's `rev()`, its items the same, from its end (#356).
        if let ExprKind::Call { fun, ref args, .. } = thir[head].kind
            && let Some((def_id, _)) = fn_def(thir[strip(thir, fun)].ty)
            && is_iterator_rev(self.tcx, def_id)
            && let [range] = args[..]
        {
            head = strip(thir, range);
        }
        // A slice's items' `enumerate()`, its index below the slice's
        // length, which the slice's `iter()` holds borrowed (#363).
        let enumerated = match thir[head].kind {
            ExprKind::Call { fun, ref args, .. }
                if let Some((def_id, _)) = fn_def(thir[strip(thir, fun)].ty)
                    && is_iterator_enumerate(self.tcx, def_id)
                    && let [items] = args[..]
                    && let ExprKind::Call {
                        fun: items_fun,
                        args: ref of,
                        ..
                    } = thir[strip(thir, items)].kind
                    && let Some((items_def, _)) = fn_def(thir[strip(thir, items_fun)].ty)
                    && is_slice_items(self.tcx, items_def)
                    && let [slice] = of[..] =>
            {
                self.slice(slice).map(|slice| (slice, self.length_most_of(items_fun)))
            }
            _ => None,
        };
        // Its start and end, and how far below its end its last item is:
        // `a..b`'s 1, `a..=b`'s, `RangeInclusive::new(a, b)`, 0.
        let bounds = match thir[head].kind {
            ExprKind::Adt(ref range) if self.tcx.is_lang_item(range.adt_def.did(), LangItem::Range) => {
                let [start, end] = &range.fields[..] else {
                    unreachable!("a range's two fields")
                };
                Some((start.expr, end.expr, 1))
            }
            ExprKind::Call { fun, ref args, .. }
                if let Some((def_id, _)) = fn_def(thir[strip(thir, fun)].ty)
                    && self.tcx.is_lang_item(def_id, LangItem::RangeInclusiveNew)
                    && let [start, end] = args[..] =>
            {
                Some((start, end, 0))
            }
            _ => None,
        };
        let body = thir[for_loop.body].span.source_callsite();
        let each = match bounds {
            Some((start_at, end_at, past)) => {
                let start = self.expr(start_at, known);
                let end = self.expr(end_at, known);
                let end = end.map(|r| self.bounded_by_itself(r, end_at));
                start.zip(end).map(|(start, end)| Range {
                    lo: start.lo,
                    hi: end.hi - past,
                    below: (end.below).and_then(|(slice, b)| Some((self.unchanged_in(body, slice)?, b.offset(-past)))),
                })
            }
            None => {
                self.expr(for_loop.head, known);
                enumerated.map(|(slice, most)| Range {
                    lo: 0,
                    hi: most - 1,
                    below: self
                        .unchanged_in(body, slice)
                        .map(|slice| (slice, Below::LENGTH.offset(-1))),
                })
            }
        };
        self.targets.push(Target {
            scope: for_loop.scope,
            breaks: None,
            continues: None,
        });
        self.walk_loop(for_loop.body, Some((for_loop.pat, each)), known);
        let target = self.targets.pop().expect("pushed");
        *known = either(known.take(), target.breaks);
    }

    /// `slice`, where nothing in `body`, a loop's, changes it: what a bound
    /// by its length holds of each time round.
    fn unchanged_in(&self, body: rustc_span::Span, slice: Slice) -> Option<Slice> {
        let changed = (self.changes.get(&slice.1)).is_some_and(|at| at.iter().any(|at| body.contains(*at)));
        (!changed).then_some(slice)
    }

    /// `l op r` of integers in `(lo, hi)`, at `site`: its range, and whether
    /// it stays in them, which its site's judgement keeps. `of`, the
    /// variables `l` and `r` are where they're one (`value_base`): `n + k`
    /// is at most `n + k` by `n`'s value, and `n - i` of an `i ≤ n + b` at
    /// least `-b`.
    fn op(
        &mut self,
        site: ExprId,
        op: BinOp,
        (l, r): (&Range, &Range),
        (lo, hi): (i128, i128),
        of: (Option<Slice>, Option<Slice>),
    ) -> Range {
        let bound = |r: &Range, by: &dyn Fn(Below) -> Below| r.below.as_ref().map(|(s, x)| (s.clone(), by(*x)));
        let by_itself = |base: &Option<Slice>, by: i128| base.clone().map(|base| (base, Below::LENGTH.offset(by)));
        let made = match op {
            BinOp::Add => Some(Range {
                lo: l.lo + r.lo,
                hi: l.hi + r.hi,
                below: match (&l.below, &r.below) {
                    (Some((s, x)), Some((t, y))) if s == t => Some((s.clone(), x.plus(*y))),
                    (Some(_), _) => bound(l, &|x| x.offset(r.hi)),
                    (None, Some(_)) => bound(r, &|y| y.offset(l.hi)),
                    (None, None) => by_itself(&of.0, r.hi).or_else(|| by_itself(&of.1, l.hi)),
                },
            }),
            BinOp::Sub => {
                // `n - i` of an `i` at most `n + b`: at least `-b`.
                let floor = match (&of.0, &r.below) {
                    (Some(n), Some((base, x))) if base == n && x.c == 1 && x.s == 0 => -x.b,
                    _ => i128::MIN,
                };
                Some(Range {
                    lo: (l.lo - r.hi).max(floor),
                    hi: l.hi - r.lo,
                    below: bound(l, &|x| x.offset(-r.lo)).or_else(|| by_itself(&of.0, -r.lo)),
                })
            }
            BinOp::Mul => {
                let products = [l.lo * r.lo, l.lo * r.hi, l.hi * r.lo, l.hi * r.hi];
                Some(Range {
                    lo: *products.iter().min().expect("four"),
                    hi: *products.iter().max().expect("four"),
                    below: match (r.constant(), l.constant()) {
                        (Some(k), _) if k >= 0 => bound(l, &|x| x.times(k)),
                        (_, Some(k)) if k >= 0 => bound(r, &|y| y.times(k)),
                        _ => None,
                    },
                })
            }
            BinOp::Shr if l.lo >= 0 => Some(match r.constant() {
                Some(k @ 0..32) => Range {
                    lo: l.lo >> k,
                    hi: l.hi >> k,
                    below: bound(l, &|x| Below { s: x.s + k as u32, ..x }.lowest()).filter(|(_, x)| x.s <= 64),
                },
                _ => Range::of(0, l.hi),
            }),
            BinOp::BitAnd if l.lo >= 0 && r.lo >= 0 => Some(Range::of(0, l.hi.min(r.hi))),
            BinOp::Div if l.lo >= 0 && r.lo > 0 => Some(Range::of(l.lo / r.hi, l.hi / r.lo)),
            BinOp::Rem if l.lo >= 0 && r.lo > 0 => Some(Range::of(0, (r.hi - 1).min(l.hi))),
            _ => None,
        };
        let within = made.clone().filter(|m| m.lo >= lo && m.hi <= hi);
        // Of constants alone, one of them named, `usize::BITS - 1`: a
        // constant, whose wrap is folded where it's written, `31`, not
        // judged (ADR 0720).
        let thir = self.thir;
        let named = |e: ExprId| matches!(thir[strip(thir, e)].kind, ExprKind::NamedConst { .. });
        let constant = l.constant().is_some()
            && r.constant().is_some()
            && matches!(thir[site].kind, ExprKind::Binary { lhs, rhs, .. } | ExprKind::AssignOp { lhs, rhs, .. }
                if named(lhs) || named(rhs));
        if matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul) && !constant {
            *self.ops.entry(site).or_insert(true) &=
                within.is_some() && !(op == BinOp::Mul && l.may_make_negative_zero(r));
        }
        // What wraps is anything in its type; a sum that can only pass its
        // most, of an operand not below 0, still at most its bound, as what
        // wraps is less.
        match (within, made) {
            (Some(within), _) => within,
            (None, Some(made)) if op == BinOp::Add && (l.lo >= 0 || r.lo >= 0) => Range {
                below: made.below,
                ..Range::of(lo, hi)
            },
            _ => Range::of(lo, hi),
        }
    }

    /// `items[range]` at `site`, of a `range` written `start..end`,
    /// `..end`, `start..` or `..`: in bounds where `start ≤ end ≤` the
    /// slice's length each time it's reached, so it can't panic (ADR 0806).
    fn judge_range(&mut self, site: ExprId, items: ExprId, range: ExprId, known: &Option<Known>) {
        let Some(k) = known else { return };
        let thir = self.thir;
        let ok = match (self.slice(items), &thir[strip(thir, range)].kind) {
            (Some(slice), ExprKind::Adt(adt)) => {
                let variant = adt.adt_def.non_enum_variant();
                let part = |name: &str| {
                    (adt.fields.iter())
                        .find(|field| variant.fields[field.name].name.as_str() == name)
                        .map(|field| field.expr)
                };
                // An array's, which its type says.
                let fixed = match thir[items].ty.peel_refs().kind() {
                    ty::Array(_, n) => n.try_to_target_usize(self.tcx).map(i128::from),
                    _ => None,
                };
                // At most the length: by its bound, or below its least.
                let up_to = |r: &Range| {
                    r.lo >= 0
                        && (r.below.as_ref().is_some_and(|(s, b)| *s == slice && b.up_to())
                            || k.lengths.get(&slice).is_some_and(|&least| r.hi <= least)
                            || fixed.is_some_and(|n| r.hi <= n))
                };
                let start = match part("start") {
                    Some(start) => self.value_of(start, known),
                    None => Some(Range::of(0, 0)),
                };
                match (start, part("end")) {
                    (Some(start), Some(end)) => self
                        .value_of(end, known)
                        .is_some_and(|end| up_to(&end) && start.lo >= 0 && start.hi <= end.lo),
                    (Some(start), None) => up_to(&start),
                    (None, _) => false,
                }
            }
            _ => false,
        };
        *self.indexes.entry(site).or_insert(true) &= ok;
    }

    /// The range of `e`, which reads without changing anything, where
    /// `known` is: walked again, as a condition's sides are, judging
    /// nothing.
    fn value_of(&mut self, e: ExprId, known: &Option<Known>) -> Option<Range> {
        if !pure(self.tcx, self.thir, e) {
            return None;
        }
        let judged = (
            std::mem::take(&mut self.indexes),
            std::mem::take(&mut self.ops),
            std::mem::take(&mut self.casts),
        );
        let mut quiet = known.clone();
        let value = self.expr(e, &mut quiet);
        (self.indexes, self.ops, self.casts) = judged;
        value
    }

    /// `items[index]` at `site`: in bounds where the index is below the
    /// slice's length each time it's reached.
    fn judge_index(&mut self, site: ExprId, items: ExprId, index: Option<Range>, known: &Option<Known>) {
        let Some(k) = known else { return };
        let ok = self.slice(items).zip(index).is_some_and(|(slice, index)| {
            index.lo >= 0
                && (index
                    .below
                    .as_ref()
                    .is_some_and(|(s, b)| k.same_length(s, &slice) && b.within())
                    || k.lengths.get(&slice).is_some_and(|&least| index.hi < least))
        });
        *self.indexes.entry(site).or_insert(true) &= ok;
    }
}
