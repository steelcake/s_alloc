use crate::valiating_alloc::ValidatingAllocator;
use std::alloc::{AllocError, Allocator, Layout};
use std::cell::RefCell;
use std::ptr::NonNull;

use crate::{
    bump_alloc::{self, BumpAlloc},
    local_alloc::{self, LocalAlloc},
};

#[test]
fn test_local_alloc() {
    let alloc = LocalAlloc::new(local_alloc::Config::new(&std::alloc::Global));
    test_allocator_all(alloc);
}

#[test]
fn test_bump_alloc() {
    let alloc = BumpAlloc::new(bump_alloc::Config::new(std::alloc::Global));
    test_allocator_all(alloc);
}

#[test]
fn test_local_bump_alloc() {
    let alloc = LocalAlloc::new(local_alloc::Config::new(&std::alloc::Global));
    let alloc = BumpAlloc::new(bump_alloc::Config::new(&alloc));
    test_allocator_all(alloc);
}

fn test_allocator<Alloc: Allocator>(alloc: Alloc) {
    let alloc = ValidatingAllocator::new(alloc);
}

fn test_allocator_aligned<Alloc: Allocator>(alloc: Alloc) {}

fn test_allocator_large_alignment<Alloc: Allocator>(alloc: Alloc) {}

fn test_allocator_aligned_shrink<Alloc: Allocator>(alloc: Alloc) {}

fn test_allocator_all<Alloc: Allocator>(alloc: Alloc) {
    test_allocator(&alloc);
    test_allocator_aligned(&alloc);
    test_allocator_large_alignment(&alloc);
    test_allocator_aligned_shrink(&alloc);
}
