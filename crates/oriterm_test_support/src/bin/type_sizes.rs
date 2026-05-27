//! Print `size_of` for the layout-sensitive `oriterm_core` types that carry a
//! `const _: () = assert!(size_of::<T>() ...)` pin (`Cell`, `RenderableCell`).
//!
//! Use when writing or updating a size assertion to read the concrete byte size
//! directly, instead of the array-length compile-error trick
//! (`const _: [u8; 0] = [0u8; size_of::<T>()];`).
//!
//! `cargo run -p oriterm_test_support --bin type-sizes`

use oriterm_core::{Cell, CellExtra, CellFlags, RenderableCell};

fn main() {
    let rows: &[(&str, usize)] = &[
        ("Cell", size_of::<Cell>()),
        ("CellExtra", size_of::<CellExtra>()),
        ("CellFlags", size_of::<CellFlags>()),
        ("RenderableCell", size_of::<RenderableCell>()),
    ];
    let width = rows.iter().map(|(name, _)| name.len()).max().unwrap_or(0);
    for (name, size) in rows {
        println!("{name:<width$}  {size:>4} bytes");
    }
}
