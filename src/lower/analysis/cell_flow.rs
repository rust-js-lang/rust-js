//! Which `RefCell`s each of the crate's borrows can be of (ADR 0863,
//! amended, issue #445): a value-flow of its cells, where each is made and
//! where it goes, by unification over its bodies' MIR, Steensgaard's.
//!
//! Each value a body holds is a node; what it's made of, a field, what a
//! reference points at, what a `Vec` or an `Rc` holds, a node of its own
//! beneath it. A move, a copy, a reference, a call of the crate's and its
//! return make two nodes one, and with them what's beneath them. Std's code
//! moves a `T` only where a `T` goes, so a call of std's makes each value of
//! a type it's given or gives one with each other of that type; so does a
//! binding that keeps what it's given in its collection (ADR 1050).
//!
//! What code rust-js can't see is given, and what it gives, is `TOP`: a
//! binding's, a library's, a `dyn`'s or a fn pointer's call; what an
//! exported function, a trait's method or a closure that code may call is
//! given and gives; a raw pointer's cast, a transmute; a static. Each node
//! beneath `TOP` is a cell that code may reach, and any such cell may be any
//! other of its type, as the crate's types said of every cell before. A
//! cell no path from `TOP` reaches is reached only by the crate's own code,
//! along the paths unification followed: one no borrow of which is counted
//! is never checked, whatever another of its type's borrows are.
//!
//! A crate any of whose bodies has no MIR here has no classes: every cell
//! is by its type, as before.

use crate::lower::Body;
use crate::lower::analysis::CellBorrows;
use crate::lower::recognition::{Recognition, Std, StdItem};
use rustc_middle::mir::{
    self, AggregateKind, BasicBlock, CastKind, Const, Local, NonDivergingIntrinsic, Operand, Place, PlaceElem, Rvalue,
    StatementKind, TerminatorKind,
};
use rustc_middle::ty::adjustment::PointerCoercion;
use rustc_middle::ty::{self, Ty, TyCtxt, TypeVisitableExt};
use rustc_span::Span;
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

/// The cells a borrow may be of: those only the crate's code reaches, by
/// their flow's class, or any a code rust-js can't see reaches, of a type
/// that may be its own.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(in crate::lower) enum CellClass {
    Own(u32),
    Anything,
}

/// Each borrow of a `RefCell` the crate's MIR makes, by its body and the
/// block its call ends, and by where it's written, the class of its cell;
/// `None`, a crate whose cells are each by its type.
#[derive(Default)]
pub(in crate::lower) struct CellFlow {
    classes: Option<Classes>,
}

#[derive(Default)]
struct Classes {
    calls: HashMap<(DefId, BasicBlock), CellClass>,
    at: HashMap<(DefId, Span, DefId), Vec<CellClass>>,
}

impl CellFlow {
    /// The class of the cell a `RefCell`'s method borrows at the call that
    /// ends `block` of `body`'s MIR, its receiver's: `None`, not known, by
    /// its type alone.
    pub(in crate::lower) fn call_class(&self, body: DefId, block: BasicBlock) -> Option<CellClass> {
        self.classes.as_ref()?.calls.get(&(body, block)).copied()
    }

    /// The class of each cell `body`'s calls of `callee` written at `span`
    /// borrow, as many as `count`, the calls its THIR has there: `None`,
    /// not known, where MIR has another number of them.
    pub(in crate::lower) fn classes_at(
        &self,
        (body, span, callee): (DefId, Span, DefId),
        count: usize,
    ) -> Option<&[CellClass]> {
        let found = self.classes.as_ref()?.at.get(&(body, span, callee))?;
        (found.len() == count).then_some(found.as_slice())
    }

    /// Each body's cells' classes (ADR 0863, amended): `borrows` says what
    /// std's code a call runs of the crate's.
    pub(in crate::lower) fn of_crate<'tcx>(
        recognition: &Recognition<'_, 'tcx>,
        borrows: &CellBorrows<'tcx>,
        bodies: &[&Body<'tcx>],
    ) -> Self {
        let tcx = recognition.tcx;
        // The crate's own bodies, functions, methods and closures, each of
        // which needs its MIR: one without it may move a cell anywhere.
        let mut own: Vec<(DefId, &crate::lower::mir::Mir<'tcx>)> = Vec::new();
        for body in bodies {
            if !body.def_id.is_local() || body.root().is_none() {
                continue;
            }
            let kind = tcx.def_kind(body.def_id);
            if !matches!(
                kind,
                rustc_hir::def::DefKind::Fn | rustc_hir::def::DefKind::AssocFn | rustc_hir::def::DefKind::Closure
            ) {
                continue;
            }
            // A thread-local's key and what its macro makes of it, its
            // storage, which only its key's accesses reach (ADR 0863,
            // amended).
            if thread_local_key(tcx, tcx.typeck_root_def_id(body.def_id)) {
                continue;
            }
            match &body.mir {
                Some(found) => own.push((body.def_id, found)),
                None => return CellFlow::default(),
            }
        }
        let mut flow = Flow {
            recognition: *recognition,
            borrows,
            graph: Graph::new(),
            index: own.iter().enumerate().map(|(i, &(id, _))| (id, i)).collect(),
            bodies: own,
            locals: HashMap::new(),
            keys: HashMap::new(),
            values_of: Vec::new(),
            seen_values: HashSet::new(),
            open: HashSet::new(),
            borrow_calls: Vec::new(),
            holds: HashMap::new(),
        };
        for at in 0..flow.bodies.len() {
            flow.body(at);
        }
        // What code rust-js can't see may call: each exported function's,
        // each trait's method's, each coroutine's, and, to a fixed point,
        // each function's and closure's that a value of reaches it.
        for at in 0..flow.bodies.len() {
            let id = flow.bodies[at].0;
            let method = tcx.trait_impl_of_assoc(id).is_some() || tcx.trait_of_assoc(id).is_some();
            if crate::lower::library::reachable(tcx, id) || method || tcx.is_coroutine(id) {
                flow.open(id);
            }
        }
        loop {
            let reached = flow.graph.reached(TOP);
            let more: Vec<DefId> = (flow.values_of.iter())
                .filter(|&&(node, id)| !flow.open.contains(&id) && reached.contains(&flow.graph.find(node)))
                .map(|&(_, id)| id)
                .collect();
            if more.is_empty() {
                break;
            }
            for id in more {
                flow.open(id);
            }
        }
        let reached = flow.graph.reached(TOP);
        let mut classes = Classes::default();
        for (body, block, span, callee, cell) in std::mem::take(&mut flow.borrow_calls) {
            let root = flow.graph.find(cell);
            let class = match reached.contains(&root) {
                true => CellClass::Anything,
                false => CellClass::Own(root),
            };
            classes.calls.insert((body, block), class);
            classes.at.entry((body, span, callee)).or_default().push(class);
        }
        CellFlow { classes: Some(classes) }
    }
}

/// The node every value code rust-js can't see has or is given is.
const TOP: u32 = 0;

/// What a node's child is of it: what it points at, a field of its
/// variant, what a std type or a JS object holds of a type parameter, by
/// the parameter's index among its generic arguments, or an array's items.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Key {
    Deref,
    Field(u32, u32),
    Arg(u32),
    Item,
}

/// Union-find of nodes, each root with its children.
struct Graph {
    parent: Vec<u32>,
    size: Vec<u32>,
    children: Vec<HashMap<Key, u32>>,
}

impl Graph {
    fn new() -> Self {
        Graph {
            parent: vec![TOP],
            size: vec![1],
            children: vec![HashMap::new()],
        }
    }

    fn fresh(&mut self) -> u32 {
        let id = self.parent.len() as u32;
        self.parent.push(id);
        self.size.push(1);
        self.children.push(HashMap::new());
        id
    }

    fn find(&mut self, node: u32) -> u32 {
        let mut root = node;
        while self.parent[root as usize] != root {
            root = self.parent[root as usize];
        }
        let mut at = node;
        while self.parent[at as usize] != root {
            let next = self.parent[at as usize];
            self.parent[at as usize] = root;
            at = next;
        }
        root
    }

    fn child(&mut self, node: u32, key: Key) -> u32 {
        let root = self.find(node);
        if let Some(&child) = self.children[root as usize].get(&key) {
            return child;
        }
        let child = self.fresh();
        self.children[root as usize].insert(key, child);
        child
    }

    /// `a` and `b` one, and what's beneath each of them by the same key.
    fn unify(&mut self, a: u32, b: u32) {
        let mut pending = vec![(a, b)];
        while let Some((a, b)) = pending.pop() {
            let (a, b) = (self.find(a), self.find(b));
            if a == b {
                continue;
            }
            // `TOP` stays its own root.
            let (keep, gone) = match (a == TOP, b == TOP, self.size[a as usize] >= self.size[b as usize]) {
                (true, _, _) | (false, false, true) => (a, b),
                _ => (b, a),
            };
            self.parent[gone as usize] = keep;
            self.size[keep as usize] += self.size[gone as usize];
            let moved = std::mem::take(&mut self.children[gone as usize]);
            for (key, child) in moved {
                match self.children[keep as usize].get(&key) {
                    Some(&theirs) => pending.push((theirs, child)),
                    None => _ = self.children[keep as usize].insert(key, child),
                }
            }
        }
    }

    /// Each root beneath `from`, and it.
    fn reached(&mut self, from: u32) -> HashSet<u32> {
        let start = self.find(from);
        let mut seen = HashSet::from([start]);
        let mut pending = vec![start];
        while let Some(at) = pending.pop() {
            let children: Vec<u32> = self.children[at as usize].values().copied().collect();
            for child in children {
                let root = self.find(child);
                if seen.insert(root) {
                    pending.push(root);
                }
            }
        }
        seen
    }
}

struct Flow<'r, 'tcx> {
    recognition: Recognition<'r, 'tcx>,
    borrows: &'r CellBorrows<'tcx>,
    graph: Graph,
    bodies: Vec<(DefId, &'r crate::lower::mir::Mir<'tcx>)>,
    index: HashMap<DefId, usize>,
    locals: HashMap<(usize, Local), u32>,
    /// Each thread-local's key's `LocalKey`, its storage.
    keys: HashMap<DefId, u32>,
    /// Each node a value of a function or closure of the crate's may be in,
    /// which code given it may call.
    values_of: Vec<(u32, DefId)>,
    seen_values: HashSet<(u32, DefId)>,
    /// The bodies code rust-js can't see may call, whose parameters and
    /// result are `TOP`.
    open: HashSet<DefId>,
    /// Each call of a `RefCell`'s that borrows it: its body, block, span,
    /// callee, and the cell's node.
    borrow_calls: Vec<(DefId, BasicBlock, Span, DefId, u32)>,
    /// Whether a value of each type may hold a cell, found when first asked.
    holds: HashMap<Ty<'tcx>, bool>,
}

/// A port of a call: a value it's given or gives, and its type.
type Port<'tcx> = (u32, Ty<'tcx>);

impl<'r, 'tcx> Flow<'r, 'tcx> {
    fn tcx(&self) -> TyCtxt<'tcx> {
        self.recognition.tcx
    }

    /// Whether a value of `ty` may hold a cell: a `RefCell` is in it, what
    /// it points at, its items, a field of its, or what may be any type, a
    /// type parameter's, a projection's, a `dyn`'s or a coroutine's.
    fn may_hold_cell(&mut self, ty: Ty<'tcx>) -> bool {
        let tcx = self.tcx();
        let ty = tcx.erase_and_anonymize_regions(ty);
        if let Some(&holds) = self.holds.get(&ty) {
            return holds;
        }
        // Within itself, a list's next node: what else it holds says.
        self.holds.insert(ty, false);
        let holds = match *ty.kind() {
            _ if self.recognition.is_std_type(ty, StdItem::RefCell) => true,
            ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Str | ty::Never => false,
            ty::FnDef(..) | ty::FnPtr(..) | ty::Foreign(_) => false,
            _ if let Some((inner, _)) = crate::lower::pointers::raw_pointee(tcx, ty) => self.may_hold_cell(inner),
            ty::Ref(_, inner, _) | ty::Array(inner, _) | ty::Slice(inner) | ty::Pat(inner, _) => {
                self.may_hold_cell(inner)
            }
            ty::Tuple(items) => items.iter().any(|item| self.may_hold_cell(item)),
            ty::Closure(_, args) => (args.as_closure().upvar_tys().iter()).any(|item| self.may_hold_cell(item)),
            ty::Adt(_, args) if self.held_by_args(ty) => args.types().any(|arg| self.may_hold_cell(arg)),
            ty::Adt(adt, args) => {
                let fields: Vec<Ty<'tcx>> = (adt.all_fields())
                    .map(|field| field.ty(tcx, args).skip_normalization())
                    .collect();
                fields
                    .into_iter()
                    .any(|field| field.has_aliases() || self.may_hold_cell(field))
            }
            _ => true,
        };
        self.holds.insert(ty, holds);
        holds
    }

    fn top(&mut self, node: u32) {
        self.graph.unify(node, TOP);
    }

    /// What code rust-js can't see may call: its parameters and result `TOP`.
    fn open(&mut self, id: DefId) {
        if !self.open.insert(id) {
            return;
        }
        let Some(&at) = self.index.get(&id) else { return };
        let count = self.bodies[at].1.body.arg_count;
        for local in 0..=count {
            let node = self.local(at, Local::from_usize(local));
            self.top(node);
        }
    }

    fn local(&mut self, at: usize, local: Local) -> u32 {
        if let Some(&node) = self.locals.get(&(at, local)) {
            return node;
        }
        let node = self.graph.fresh();
        self.locals.insert((at, local), node);
        let ty = self.bodies[at].1.body.local_decls[local].ty;
        self.typed(node, ty);
        node
    }

    /// What a node of type `ty` says of itself: a value of a type with a
    /// destructor of code that isn't std's, which runs given it, is `TOP`;
    /// and each function or closure of the crate's a value of it may be.
    fn typed(&mut self, node: u32, ty: Ty<'tcx>) {
        let tcx = self.tcx();
        if let ty::Adt(adt, _) = ty.kind()
            && !self.in_sysroot(adt.did())
            && adt.has_dtor(tcx)
        {
            self.top(node);
        }
        for part in ty.walk().filter_map(|part| part.as_type()) {
            if let ty::Closure(id, _) | ty::FnDef(id, _) | ty::Coroutine(id, _) | ty::CoroutineClosure(id, _) =
                *part.kind()
                && id.is_local()
                && self.seen_values.insert((node, id))
            {
                self.values_of.push((node, id));
            }
        }
    }

    fn typing_env(&self, at: usize) -> ty::TypingEnv<'tcx> {
        ty::TypingEnv::post_analysis(self.tcx(), self.tcx().typeck_root_def_id(self.bodies[at].0))
    }

    /// The node of `place` in body `at`.
    fn place(&mut self, at: usize, place: Place<'tcx>) -> u32 {
        let tcx = self.tcx();
        let body = &self.bodies[at].1.body;
        let mut node = self.local(at, place.local);
        let mut ty = body.local_decls[place.local].ty;
        let mut variant = 0u32;
        for elem in place.projection.iter() {
            let next = mir::PlaceTy::from_ty(ty).projection_ty(tcx, elem).ty;
            node = match elem {
                PlaceElem::Deref => match ty.kind() {
                    ty::Adt(..) if ty.is_box() => self.graph.child(node, Key::Arg(0)),
                    _ => self.graph.child(node, Key::Deref),
                },
                PlaceElem::Field(field, _) => self.field(node, ty, variant, field.as_u32()),
                PlaceElem::Index(_) | PlaceElem::ConstantIndex { .. } => self.graph.child(node, Key::Item),
                // Of the same items.
                PlaceElem::Subslice { .. } => node,
                PlaceElem::Downcast(_, at) => {
                    variant = at.as_u32();
                    ty = next;
                    continue;
                }
                PlaceElem::OpaqueCast(_) | PlaceElem::UnwrapUnsafeBinder(_) => node,
            };
            variant = 0;
            ty = next;
            self.typed(node, ty);
        }
        node
    }

    /// The node of `owner`'s field `field` of its `variant`, of type `ty`:
    /// a struct's, a tuple's, a closure's captures; of std's types and a JS
    /// object's, what it holds of the type parameter the field is, the rest
    /// `TOP` with all of it, which std's code may reach otherwise; a union's
    /// and a coroutine's, `TOP`.
    fn field(&mut self, owner: u32, ty: Ty<'tcx>, variant: u32, field: u32) -> u32 {
        let tcx = self.tcx();
        match ty.kind() {
            ty::Adt(adt, _) if adt.is_union() => {
                self.top(owner);
                owner
            }
            ty::Adt(adt, _) if self.held_by_args(ty) => {
                let declared =
                    &adt.variant(rustc_abi::VariantIdx::from_u32(variant)).fields[rustc_abi::FieldIdx::from_u32(field)];
                match tcx
                    .type_of(declared.did)
                    .instantiate_identity()
                    .skip_normalization()
                    .kind()
                {
                    ty::Param(param) => self.graph.child(owner, Key::Arg(param.index)),
                    _ => {
                        self.top(owner);
                        owner
                    }
                }
            }
            ty::Adt(..) | ty::Tuple(_) | ty::Closure(..) => self.graph.child(owner, Key::Field(variant, field)),
            _ => {
                self.top(owner);
                owner
            }
        }
    }

    /// Whether what a value of `ty` holds is by its type's parameters, not
    /// its fields: std's types, whose fields are their own, and a JS object,
    /// which has none.
    fn held_by_args(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.in_sysroot(adt.did())) || self.recognition.is_js_object(ty)
    }

    /// Whether `id` is std's, or a crate's its sysroot holds beside it.
    fn in_sysroot(&self, id: DefId) -> bool {
        !id.is_local() && self.recognition.in_sysroot(id)
    }

    fn operand(&mut self, at: usize, operand: &Operand<'tcx>) -> u32 {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => self.place(at, *place),
            Operand::Constant(constant) => self.constant(at, constant),
            Operand::RuntimeChecks(_) => self.graph.fresh(),
        }
    }

    /// A constant's node: a thread-local's key, its storage's; a reference
    /// to one; a static's, which code rust-js can't see may reach, `TOP`;
    /// any other that may hold a cell, `TOP`; else a value of its own.
    fn constant(&mut self, at: usize, constant: &mir::ConstOperand<'tcx>) -> u32 {
        let tcx = self.tcx();
        let ty = constant.ty();
        if !self.may_hold_cell(ty) {
            // A function's own, `iter.map(f)`'s `f`, which what it's given
            // to may call.
            let node = self.graph.fresh();
            self.typed(node, ty);
            return node;
        }
        let key_of = |c: &Const<'tcx>| match c {
            Const::Unevaluated(uv, _) if uv.promoted.is_none() && thread_local_key(tcx, uv.def) => Some(uv.def),
            _ => None,
        };
        if let Some(key) = key_of(&constant.const_) {
            return self.key(key);
        }
        if let Const::Unevaluated(uv, _) = constant.const_
            && let Some(promoted) = uv.promoted
            && uv.def == self.bodies[at].0
            && let Some(found) = self.promoted(at, promoted)
            && let Some(key) = key_of(&found.const_)
        {
            let node = self.graph.fresh();
            let key = self.key(key);
            let pointee = self.graph.child(node, Key::Deref);
            self.graph.unify(pointee, key);
            return node;
        }
        TOP
    }

    /// The constant a body's promoted constant `promoted` refers to, `&KEY`'s
    /// `KEY`.
    fn promoted(&self, at: usize, promoted: mir::Promoted) -> Option<mir::ConstOperand<'tcx>> {
        crate::lower::mir::referred_constant(self.bodies[at].1.promoted.get(promoted)?)
    }

    /// A thread-local's key's node, the `LocalKey` that holds its storage:
    /// a `pub` one's `TOP`, which a consumer reaches; what its `init` gives,
    /// what its storage holds.
    fn key(&mut self, key: DefId) -> u32 {
        if let Some(&node) = self.keys.get(&key) {
            return node;
        }
        let tcx = self.tcx();
        let node = self.graph.fresh();
        self.keys.insert(key, node);
        if tcx.visibility(key).is_public() {
            self.top(node);
        }
        let held = self.graph.child(node, Key::Arg(0));
        let inits: Vec<usize> = (0..self.bodies.len())
            .filter(|&at| tcx.opt_parent(self.bodies[at].0) == Some(key))
            .collect();
        for at in inits {
            let given = self.local(at, mir::RETURN_PLACE);
            self.graph.unify(given, held);
        }
        node
    }

    fn body(&mut self, at: usize) {
        let body = &self.bodies[at].1.body;
        for (block, data) in body.basic_blocks.iter_enumerated() {
            for statement in &data.statements {
                match &statement.kind {
                    StatementKind::Assign(assign) => {
                        let (place, rvalue) = &**assign;
                        let into = self.place(at, *place);
                        self.rvalue(at, into, rvalue);
                    }
                    StatementKind::Intrinsic(intrinsic) => {
                        if let NonDivergingIntrinsic::CopyNonOverlapping(copy) = &**intrinsic {
                            for operand in [&copy.src, &copy.dst] {
                                let node = self.operand(at, operand);
                                self.top(node);
                            }
                        }
                    }
                    _ => {}
                }
            }
            match &data.terminator().kind {
                TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } => {
                    let given: Vec<&Operand<'tcx>> = args.iter().map(|arg| &arg.node).collect();
                    let gives = Some(*destination);
                    let span = data.terminator().source_info.span;
                    self.call(at, (block, span), func, &given, gives);
                }
                TerminatorKind::TailCall { func, args, .. } => {
                    let given: Vec<&Operand<'tcx>> = args.iter().map(|arg| &arg.node).collect();
                    let span = data.terminator().source_info.span;
                    self.call(at, (block, span), func, &given, Some(Place::return_place()));
                }
                TerminatorKind::Yield { value, resume_arg, .. } => {
                    let value = self.operand(at, value);
                    self.top(value);
                    let resumed = self.place(at, *resume_arg);
                    self.top(resumed);
                }
                TerminatorKind::InlineAsm { operands, .. } => {
                    for operand in operands.iter() {
                        use rustc_middle::mir::InlineAsmOperand as Asm;
                        let places: Vec<Place<'tcx>> = match operand {
                            Asm::In { value, .. } => value.place().into_iter().collect(),
                            Asm::Out { place, .. } => place.iter().copied().collect(),
                            Asm::InOut {
                                in_value, out_place, ..
                            } => in_value.place().into_iter().chain(out_place.iter().copied()).collect(),
                            _ => Vec::new(),
                        };
                        for place in places {
                            let node = self.place(at, place);
                            self.top(node);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn rvalue(&mut self, at: usize, into: u32, rvalue: &Rvalue<'tcx>) {
        let tcx = self.tcx();
        let body = &self.bodies[at].1.body;
        match rvalue {
            Rvalue::Use(operand, _) | Rvalue::WrapUnsafeBinder(operand, _) => {
                let from = self.operand(at, operand);
                self.graph.unify(into, from);
            }
            Rvalue::CopyForDeref(place) => {
                let from = self.place(at, *place);
                self.graph.unify(into, from);
            }
            Rvalue::Repeat(operand, _) => {
                let from = self.operand(at, operand);
                let items = self.graph.child(into, Key::Item);
                self.graph.unify(items, from);
            }
            Rvalue::Ref(_, _, place) | Rvalue::RawPtr(_, place) | Rvalue::Reborrow(_, _, place) => {
                let from = self.place(at, *place);
                let pointee = self.graph.child(into, Key::Deref);
                self.graph.unify(pointee, from);
            }
            Rvalue::ThreadLocalRef(_) => self.top(into),
            Rvalue::Cast(kind, operand, to) => {
                let from = self.operand(at, operand);
                let same = match kind {
                    CastKind::PointerCoercion(PointerCoercion::Unsize, _) => {
                        // Made a `dyn`, it's what any code may be given.
                        !to.walk().filter_map(|part| part.as_type()).any(|part| part.is_trait())
                    }
                    CastKind::PointerCoercion(PointerCoercion::MutToConstPointer, _) | CastKind::Subtype => true,
                    // Scalars, which hold no cell.
                    CastKind::IntToInt | CastKind::FloatToInt | CastKind::FloatToFloat | CastKind::IntToFloat => {
                        return;
                    }
                    // A function made a pointer, which code rust-js can't see
                    // may call; a pointer of another type, a transmute.
                    _ => false,
                };
                match same {
                    true => self.graph.unify(into, from),
                    false => {
                        self.top(from);
                        self.top(into);
                    }
                }
            }
            Rvalue::BinaryOp(_, operands) => {
                let ty = rvalue.ty(body, tcx);
                if self.may_hold_cell(ty) || operands.0.ty(body, tcx).is_any_ptr() {
                    for operand in [&operands.0, &operands.1] {
                        let node = self.operand(at, operand);
                        self.top(node);
                    }
                    self.top(into);
                }
            }
            Rvalue::UnaryOp(_, operand) => {
                if self.may_hold_cell(rvalue.ty(body, tcx)) {
                    let node = self.operand(at, operand);
                    self.top(node);
                    self.top(into);
                }
            }
            Rvalue::Discriminant(_) => {}
            Rvalue::Aggregate(kind, operands) => {
                let given: Vec<u32> = operands.iter().map(|operand| self.operand(at, operand)).collect();
                match **kind {
                    AggregateKind::Array(_) => {
                        let items = self.graph.child(into, Key::Item);
                        for node in given {
                            self.graph.unify(items, node);
                        }
                    }
                    AggregateKind::Tuple | AggregateKind::Closure(..) => {
                        for (i, node) in given.into_iter().enumerate() {
                            let field = self.graph.child(into, Key::Field(0, i as u32));
                            self.graph.unify(field, node);
                        }
                    }
                    AggregateKind::Adt(adt, variant, args, _, active) => {
                        let ty = Ty::new_adt(tcx, tcx.adt_def(adt), args);
                        for (i, node) in given.into_iter().enumerate() {
                            let field = active.map_or(i as u32, |active| active.as_u32());
                            let place = self.field(into, ty, variant.as_u32(), field);
                            self.graph.unify(place, node);
                        }
                    }
                    AggregateKind::Coroutine(..) | AggregateKind::CoroutineClosure(..) | AggregateKind::RawPtr(..) => {
                        for node in given {
                            self.top(node);
                        }
                        self.top(into);
                    }
                }
            }
        }
    }

    /// A call of `func` of `given`, which gives `gives`, ending `block`.
    fn call(
        &mut self,
        at: usize,
        (block, span): (BasicBlock, Span),
        func: &Operand<'tcx>,
        given: &[&Operand<'tcx>],
        gives: Option<Place<'tcx>>,
    ) {
        let tcx = self.tcx();
        let body = &self.bodies[at].1.body;
        let mut ports: Vec<Port<'tcx>> = given
            .iter()
            .map(|operand| (self.operand(at, operand), operand.ty(body, tcx)))
            .collect();
        if let Some(place) = gives {
            ports.push((self.place(at, place), place.ty(body, tcx).ty));
        }
        let Some((id, args)) = crate::lower::fn_def(func.ty(body, tcx)) else {
            return self.unseen(&ports);
        };
        // A borrow of a `RefCell`, of the cell its receiver points at.
        let recognition = Recognition {
            typing_env: self.typing_env(at),
            ..self.recognition
        };
        if let Some(&(receiver, receiver_ty)) = ports.first()
            && let Some(cell) = self.cell_of(receiver, receiver_ty)
            && matches!(
                recognition.classify(id, args),
                Some(
                    Std::Borrow { lock: false, .. }
                        | Std::TryBorrow { lock: false, .. }
                        | Std::CellReplace
                        | Std::CellTake
                        | Std::CellReplaceWith
                        | Std::CellSwap
                        | Std::CellGetMut
                )
            )
        {
            self.borrow_calls.push((self.bodies[at].0, block, span, id, cell));
        }
        if crate::lower::bindings::is_binding(tcx, id) {
            return match crate::lower::bindings::keeps_in_collection(tcx, id) {
                // What it keeps is its collection's, as std's code keeps it
                // (ADR 1050).
                true => self.by_type(at, &ports, true),
                false => self.unseen(&ports),
            };
        }
        let Ok(Some(instance)) = crate::lower::items::resolve_instance_in(tcx, recognition.typing_env, id, args) else {
            return self.unseen(&ports);
        };
        let ty::InstanceKind::Item(callee) = instance.def else {
            // A shim of a closure's, which runs it given what it's given.
            if instance.def_id().is_local() {
                self.open(instance.def_id());
            }
            return self.unseen(&ports);
        };
        if let Some(&theirs) = self.index.get(&callee) {
            return self.call_body(theirs, &ports);
        }
        if !self.recognition.is_std(callee) || tcx.fn_sig(callee).skip_binder().safety().is_unsafe() {
            if callee.is_local() {
                self.open(callee);
            }
            return self.unseen(&ports);
        }
        // Std's, and what it runs of the crate's: each body's own ports.
        let Some(runs) = self.borrows.std_runs(&recognition, callee, instance.args) else {
            return self.unseen(&ports);
        };
        for (run, run_args) in runs {
            let (Some(&theirs), Some(run_args)) = (self.index.get(&run), run_args) else {
                if run.is_local() {
                    self.open(run);
                }
                return self.unseen(&ports);
            };
            let mir = &self.bodies[theirs].1.body;
            for local in 0..=mir.arg_count {
                let local = Local::from_usize(local);
                let ty = ty::EarlyBinder::bind(tcx, mir.local_decls[local].ty)
                    .instantiate(tcx, run_args)
                    .skip_normalization();
                ports.push((self.local(theirs, local), ty));
            }
        }
        self.by_type(at, &ports, false)
    }

    /// The cell `node`, of type `ty`, a reference to a `RefCell` or one to
    /// a reference to one, points at.
    fn cell_of(&mut self, mut node: u32, mut ty: Ty<'tcx>) -> Option<u32> {
        while let ty::Ref(_, inner, _) = ty.kind() {
            node = self.graph.child(node, Key::Deref);
            ty = *inner;
        }
        self.recognition.is_std_type(ty, StdItem::RefCell).then_some(node)
    }

    /// Each port `TOP`: what code rust-js can't see is given and gives.
    fn unseen(&mut self, ports: &[Port<'tcx>]) {
        for &(node, _) in ports {
            self.top(node);
        }
    }

    /// A call of the crate's body `theirs` of `ports`, its arguments and
    /// the place it gives, each one with its parameter or its result; a
    /// closure's called by an `Fn` trait's method, of its captures and a
    /// tuple of its arguments.
    fn call_body(&mut self, theirs: usize, ports: &[Port<'tcx>]) {
        let tcx = self.tcx();
        let (id, mir) = (self.bodies[theirs].0, &self.bodies[theirs].1.body);
        let count = mir.arg_count;
        let Some((&result, given)) = ports.split_last() else {
            return;
        };
        let params: Vec<(u32, Ty<'tcx>)> = (1..=count)
            .map(|local| {
                let local = Local::from_usize(local);
                (self.local(theirs, local), mir.local_decls[local].ty)
            })
            .collect();
        let returned = self.local(theirs, mir::RETURN_PLACE);
        self.graph.unify(result.0, returned);
        if tcx.is_closure_like(id) && given.len() == 2 && !params.is_empty() {
            let (env, env_ty) = params[0];
            self.same_value((given[0].0, given[0].1), (env, env_ty));
            match mir.spread_arg {
                // Its arguments' tuple, as it's given.
                Some(_) if params.len() == 2 => self.graph.unify(given[1].0, params[1].0),
                Some(_) => self.unseen(ports),
                None => {
                    for (i, &(param, _)) in params.iter().enumerate().skip(1) {
                        let item = self.graph.child(given[1].0, Key::Field(0, (i - 1) as u32));
                        self.graph.unify(item, param);
                    }
                }
            }
            return;
        }
        if given.len() != params.len() {
            for &(node, _) in ports {
                self.top(node);
            }
            self.open(id);
            return;
        }
        for (&(arg, arg_ty), &(param, param_ty)) in given.iter().zip(&params) {
            self.same_value((arg, arg_ty), (param, param_ty));
        }
    }

    /// Two nodes of one value, either a reference to it, a closure's
    /// captures as `Fn::call` gives them.
    fn same_value(&mut self, (a, a_ty): Port<'tcx>, (b, b_ty): Port<'tcx>) {
        match (a_ty.is_ref(), b_ty.is_ref()) {
            (true, false) => {
                let pointee = self.graph.child(a, Key::Deref);
                self.graph.unify(pointee, b);
            }
            (false, true) => {
                let pointee = self.graph.child(b, Key::Deref);
                self.graph.unify(a, pointee);
            }
            _ => self.graph.unify(a, b),
        }
    }

    /// A call of code that moves a value only where one of its type goes,
    /// std's, or a binding that keeps what it's given in its collection:
    /// each node of a type among `ports` one with each other of it. One
    /// that may move what it's given otherwise, of a raw pointer, a fn
    /// pointer, a coroutine, or, of std's, a `dyn`, which std may make any
    /// type, is what code rust-js can't see does: `TOP`.
    fn by_type(&mut self, at: usize, ports: &[Port<'tcx>], collection: bool) {
        let mut found: Vec<(Ty<'tcx>, u32)> = Vec::new();
        for &(node, ty) in ports {
            if !self.walk(at, (node, ty), collection, &mut Vec::new(), &mut found) {
                return self.unseen(ports);
            }
        }
        let mut first: HashMap<Ty<'tcx>, u32> = HashMap::new();
        for (ty, node) in found {
            match first.get(&ty) {
                Some(&other) => self.graph.unify(other, node),
                None => _ = first.insert(ty, node),
            }
        }
    }

    /// Each node beneath `node`, of type `ty`, by its type, that may hold a
    /// cell, into `found`; `false` where its type is one std's code or a
    /// collection may move otherwise than as its type says.
    fn walk(
        &mut self,
        at: usize,
        (node, ty): Port<'tcx>,
        collection: bool,
        within: &mut Vec<(Ty<'tcx>, u32)>,
        found: &mut Vec<(Ty<'tcx>, u32)>,
    ) -> bool {
        let tcx = self.tcx();
        let typing_env = self.typing_env(at);
        let ty = tcx.erase_and_anonymize_regions(ty);
        let ty = tcx
            .try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty))
            .unwrap_or(ty);
        if !self.may_hold_cell(ty) {
            return true;
        }
        // A type within itself, a list's next node: the same node.
        if let Some(&(_, outer)) = within.iter().find(|(outer, _)| *outer == ty) {
            self.graph.unify(outer, node);
            return true;
        }
        if within.len() > 32 {
            return false;
        }
        self.typed(node, ty);
        found.push((ty, node));
        within.push((ty, node));
        let mut parts: Vec<(Key, Ty<'tcx>)> = Vec::new();
        let walked = match *ty.kind() {
            ty::Ref(_, inner, _) => {
                parts.push((Key::Deref, inner));
                true
            }
            // A pointer's, which may be cast to point at anything.
            _ if crate::lower::pointers::raw_pointee(tcx, ty).is_some() => false,
            ty::FnPtr(..) | ty::Coroutine(..) | ty::CoroutineClosure(..) => false,
            ty::Dynamic(..) => {
                // A collection keeps what it's given whatever it is: what a
                // `dyn` it's given gives, by the types its trait names.
                let named = ty.walk().skip(1).filter_map(|part| part.as_type());
                parts.extend(named.enumerate().map(|(i, part)| (Key::Arg(i as u32), part)));
                collection
            }
            ty::Array(item, _) | ty::Slice(item) => {
                parts.push((Key::Item, item));
                true
            }
            ty::Tuple(items) => {
                parts.extend(
                    items
                        .iter()
                        .enumerate()
                        .map(|(i, item)| (Key::Field(0, i as u32), item)),
                );
                true
            }
            ty::Adt(_, args) if self.held_by_args(ty) => {
                for (i, arg) in args.iter().enumerate() {
                    if let Some(arg) = arg.as_type() {
                        parts.push((Key::Arg(i as u32), arg));
                    }
                }
                true
            }
            ty::Adt(adt, _) if adt.is_union() => false,
            // A struct's, an enum's or a closure's own: what it holds is
            // reached only by its own code, a body of the crate's, which its
            // own nodes follow, or one std's code runs by a bound, whose
            // ports are the call's too. Std's code moves it whole.
            ty::Adt(..) | ty::Closure(..) => true,
            // What may be any type, of which nothing's known beneath it.
            _ => true,
        };
        if !walked {
            within.pop();
            return false;
        }
        for (key, part) in parts {
            let child = self.graph.child(node, key);
            if !self.walk(at, (child, part), collection, within, found) {
                within.pop();
                return false;
            }
        }
        within.pop();
        true
    }
}

/// Whether `id` is a thread-local's key, the `const` of a `LocalKey` its
/// macro makes.
fn thread_local_key(tcx: TyCtxt<'_>, id: DefId) -> bool {
    matches!(tcx.def_kind(id), rustc_hir::def::DefKind::Const { .. })
        && matches!(tcx.type_of(id).instantiate_identity().skip_normalization().kind(),
            ty::Adt(key, _) if crate::lower::recognition::is_std_def(tcx, key.did(), StdItem::LocalKey))
}
