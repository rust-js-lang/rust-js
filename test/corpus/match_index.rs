// A `match` of a fieldless enum whose every arm gives the field named as
// its variant, of one table, is the table read by the value: `MAP[kind]`,
// as react.dev's ExpandableCallout reads `variantMap[type]`.
#[derive(Clone, Copy)]
enum Kind {
    #[cfg_attr(rust_js, rust_js::name = "note")]
    Note,
    #[cfg_attr(rust_js, rust_js::name = "pitfall")]
    Pitfall,
    #[cfg_attr(rust_js, rust_js::name = "rsc")]
    Rsc,
}

struct Variant {
    title: &'static str,
    depth: u32,
}

struct Variants {
    note: Variant,
    pitfall: Variant,
    rsc: Variant,
}

static VARIANTS: Variants = Variants {
    note: Variant { title: "Note", depth: 1 },
    pitfall: Variant { title: "Pitfall", depth: 2 },
    rsc: Variant { title: "React Server Components", depth: 3 },
};

fn variant(kind: Kind) -> &'static Variant {
    match kind {
        Kind::Note => &VARIANTS.note,
        Kind::Pitfall => &VARIANTS.pitfall,
        Kind::Rsc => &VARIANTS.rsc,
    }
}

// A `let` of it, as ExpandableCallout has `const variant = variantMap[type]`.
fn titled(kind: Kind) -> &'static str {
    let chosen = match kind {
        Kind::Note => &VARIANTS.note,
        Kind::Pitfall => &VARIANTS.pitfall,
        Kind::Rsc => &VARIANTS.rsc,
    };
    chosen.title
}

// What's matched, a call, is read once, as the key.
fn pick(i: usize) -> Kind {
    println!("pick {i}");
    [Kind::Note, Kind::Pitfall, Kind::Rsc][i % 3]
}

fn picked(i: usize) -> &'static Variant {
    match pick(i) {
        Kind::Note => &VARIANTS.note,
        Kind::Pitfall => &VARIANTS.pitfall,
        Kind::Rsc => &VARIANTS.rsc,
    }
}

// An arm of another field is a conditional, as before.
fn crossed(kind: Kind) -> &'static Variant {
    match kind {
        Kind::Note => &VARIANTS.pitfall,
        Kind::Pitfall => &VARIANTS.note,
        Kind::Rsc => &VARIANTS.rsc,
    }
}

static OTHER: Variants = Variants {
    note: Variant { title: "Other note", depth: 4 },
    pitfall: Variant { title: "Other pitfall", depth: 5 },
    rsc: Variant { title: "Other RSC", depth: 6 },
};

// Arms of two tables are a conditional too.
fn mixed(kind: Kind) -> &'static Variant {
    match kind {
        Kind::Note => &VARIANTS.note,
        Kind::Pitfall => &OTHER.pitfall,
        Kind::Rsc => &VARIANTS.rsc,
    }
}

fn main() {
    for kind in [Kind::Note, Kind::Pitfall, Kind::Rsc] {
        let v = variant(kind);
        println!("{} {} {} {}", v.title, v.depth, crossed(kind).title, mixed(kind).title);
    }
    println!("{} {}", picked(4).title, titled(Kind::Rsc));
}
