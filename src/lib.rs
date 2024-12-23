#![feature(allocator_api)]
#![feature(alloc_layout_extra)]
#![allow(clippy::comparison_chain)]

pub mod bump_alloc;
pub mod local_alloc;
pub mod page_alloc;
mod util;
pub mod valiating_alloc;

#[cfg(test)]
mod tests;
