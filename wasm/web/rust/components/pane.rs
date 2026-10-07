// One side of the page: a heading, and a file explorer beside an editor.

use react::{JSX, jsx};

use crate::styles::HEADING;

pub struct PaneProps {
    pub title: &'static str,
    /// What the explorer is, for a screen reader: "Rust files".
    pub label: &'static str,
    pub explorer: JSX::Element,
    pub editor: JSX::Element,
}

pub fn Pane(
    PaneProps {
        title,
        label,
        explorer,
        editor,
    }: PaneProps,
) -> JSX::Element {
    jsx! {
        <section>
            <h2 className={HEADING}>{title}</h2>
            <div className="grid h-[460px] grid-cols-[150px_minmax(0,1fr)] overflow-hidden rounded-md border border-line">
                <nav className="overflow-auto border-r border-line bg-panel py-1.5 font-mono text-[13px]" aria-label={label}>
                    {explorer}
                </nav>
                {editor}
            </div>
        </section>
    }
}
