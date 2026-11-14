//! An app of the chain: a page of kit's and forms' components, each over
//! primitives, over geometry, over react; and a report of what the chain
//! computes, which native Rust prints too.

use std::fmt::Write;

use forms::{Countries, Country, Select};
use geometry::{Flip, Middleware, Offset, Rect, Shift, compute_position};
use kit::{Button, DropdownMenu, Menu, Placement, Size, Variant, button_classes};
use primitives::Collection;
use react::{JSX, jsx, use_state};

fn countries() -> Countries {
    Countries(vec![
        Country { code: "VN", name: "Vietnam" },
        Country { code: "FR", name: "France" },
        Country { code: "JP", name: "Japan" },
    ])
}

/// What the chain computes, a line each, as the native build prints it.
pub fn report() -> String {
    let mut out = String::new();
    let reference = Rect::new(100.0, 700.0, 80.0, 30.0);
    let floating = Rect::new(0.0, 0.0, 200.0, 120.0);
    let boundary = Rect::new(0.0, 0.0, 1024.0, 768.0);
    let middleware: Vec<Box<dyn Middleware>> = vec![Box::new(Offset(4.0)), Box::new(Flip), Box::new(Shift { padding: 8.0 })];
    let described: Vec<String> = middleware.iter().map(|step| step.describe()).collect();
    writeln!(out, "middleware: {}", described.join(", ")).unwrap();
    for asked in ["bottom", "bottom-start", "right-end", "top", "left-start", "middle"] {
        match asked.parse::<Placement>() {
            Ok(placement) => {
                let state = compute_position(reference, floating, boundary, placement, &middleware);
                writeln!(out, "{asked}: {} at ({}, {})", state.placement, state.point.x, state.point.y).unwrap();
            }
            Err(error) => writeln!(out, "{asked}: {error}").unwrap(),
        }
    }
    for (variant, size) in [(Variant::Default, Size::Medium), (Variant::Ghost, Size::Small), (Variant::Destructive, Size::Large)] {
        writeln!(out, "{variant:?} {size:?}: {}", button_classes(variant, size, Some("w-full"))).unwrap();
    }
    let countries = countries();
    writeln!(out, "{} countries: {}", countries.len(), countries.labels().join("; ")).unwrap();
    out
}

/// A page: a dropdown of actions, a country select, and a button.
pub fn Page() -> JSX::Element {
    let (open, set_open) = use_state(|| true);
    let (chosen, set_chosen) = use_state(|| None::<usize>);
    let placement: Placement = "bottom-start".parse().unwrap_or_default();
    let actions = jsx! { <Menu label="Actions" items={vec!["Edit".to_string(), "Delete".to_string()]} /> };
    let options = countries().labels();
    let shown = chosen.map_or("none".to_string(), |index| options[index].clone());
    jsx! {
        <main>
            <DropdownMenu
                open={*open}
                trigger="Actions"
                placement={Some(placement)}
                onOpenChange={Some(Box::new(move |open| set_open.set(open)))}
                className={Some("dropdown")}
            >
                {actions}
            </DropdownMenu>
            <Select
                label="Country"
                open={true}
                options={options}
                placement={Some(placement)}
                onChange={Some(Box::new(move |index| set_chosen.set(Some(index))))}
            />
            <p>{format!("Chosen: {shown}")}</p>
            <Button variant={Some(Variant::Outline)} size={Some(Size::Small)} className={Some("save")} type={Some("submit")}>
                {"Save"}
            </Button>
        </main>
    }
}
