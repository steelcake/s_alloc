#![feature(allocator_api)]
#![feature(slice_ptr_get)]
#![feature(alloc_layout_extra)]

#![allow(clippy::comparison_chain)]

pub mod bump_alloc;
pub mod local_alloc;
pub mod page_alloc;
mod util;

#[cfg(test)]
mod tests;
