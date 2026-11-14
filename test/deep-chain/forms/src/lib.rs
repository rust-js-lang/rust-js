//! Form fields over `primitives`, beside `kit`: a select whose options are
//! a `Collection`, in a popover placed as the caller asks.

use std::fmt;

use primitives::{Collection, Menu, Placement, Popover};
use react::{JSX, jsx};

#[derive(Clone, Debug, PartialEq)]
pub struct Country {
    pub code: &'static str,
    pub name: &'static str,
}

impl fmt::Display for Country {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.code)
    }
}

/// The countries a select offers, in the order given.
pub struct Countries(pub Vec<Country>);

impl Collection for Countries {
    type Item = Country;

    fn items(&self) -> Vec<Country> {
        self.0.clone()
    }
}

pub struct SelectProps<'a> {
    pub label: &'a str,
    pub open: bool,
    pub options: Vec<String>,
    pub placement: Option<Placement>,
    pub on_open_change: Option<Box<dyn Fn(bool)>>,
    pub on_change: Option<Box<dyn Fn(usize)>>,
}

pub fn Select(props: SelectProps) -> JSX::Element {
    let SelectProps {
        label,
        open,
        options,
        placement,
        on_open_change,
        on_change,
    } = props;
    let menu = jsx! { <Menu label={label} items={options} onSelect={on_change} /> };
    jsx! {
        <Popover open={open} trigger={label} placement={placement} onOpenChange={on_open_change} className={Some("select")}>
            {menu}
        </Popover>
    }
}
