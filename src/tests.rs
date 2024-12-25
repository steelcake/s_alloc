use crate::valiating_alloc::ValidatingAllocator;
use std::alloc::{Allocator, Layout};
use std::ptr::NonNull;

use crate::{
    bump_alloc::{self, BumpAlloc},
    local_alloc::{self, LocalAlloc},
};

#[test]
fn test_global_alloc() {
    test_allocator_all(&std::alloc::Global);
}

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
    let layout = Layout::new::<i32>().repeat(100).unwrap().0;
    let mut ptrs = Vec::<NonNull<[u8]>, &ValidatingAllocator<Alloc>>::with_capacity_in(500, &alloc);
    for _ in 0..100 {
        ptrs.push(alloc.allocate(layout).unwrap());
    }

    for _ in 0..50 {
        unsafe { alloc.deallocate(ptrs.pop().unwrap().cast::<u8>(), layout) };
    }

    ptrs.shrink_to_fit();

    for _ in 0..50 {
        unsafe { alloc.deallocate(ptrs.pop().unwrap().cast::<u8>(), layout) };
    }

    ptrs.shrink_to_fit();

    for _ in 0..100 {
        ptrs.push(alloc.allocate(layout).unwrap());
    }

    for _ in 0..100 {
        unsafe { alloc.deallocate(ptrs.pop().unwrap().cast::<u8>(), layout) };
    }

    let ptr = alloc
        .allocate(Layout::from_size_align(0, 1).unwrap())
        .unwrap();
    unsafe { alloc.deallocate(ptr.cast::<u8>(), Layout::from_size_align(0, 1).unwrap()) };
}

fn test_allocator_aligned<Alloc: Allocator>(alloc: Alloc) {
    let alloc = ValidatingAllocator::new(alloc);
    let mut aligns = Vec::<(NonNull<[u8]>, Layout), &ValidatingAllocator<Alloc>>::new_in(&alloc);
    for pow in 0..12 {
        let alignment = 1 << pow;

        let layout = Layout::from_size_align(69, alignment).unwrap();
        let ptr = alloc.allocate(layout).unwrap();

        aligns.push((ptr, layout));
    }

    for (ptr, layout) in aligns {
        unsafe { alloc.deallocate(ptr.cast::<u8>(), layout) };
    }
}

// fn test_allocator_large_alignment<Alloc: Allocator>(alloc: Alloc) {}

// fn test_allocator_aligned_shrink<Alloc: Allocator>(alloc: Alloc) {}

fn test_allocator_all<Alloc: Allocator>(alloc: Alloc) {
    test_allocator(&alloc);
    test_allocator_aligned(&alloc);
    // test_allocator_large_alignment(&alloc);
    // test_allocator_aligned_shrink(&alloc);
}
