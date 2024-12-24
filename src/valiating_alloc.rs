use std::alloc::{AllocError, Allocator, Layout};
use std::cell::RefCell;
use std::ptr::NonNull;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Slice {
    ptr: usize,
    len: usize,
}

pub struct ValidatingAllocator<Alloc: Allocator> {
    inner: Alloc,
    alive_allocs: RefCell<Vec<Slice>>,
}

impl<Alloc: Allocator> ValidatingAllocator<Alloc> {
    pub fn new(inner: Alloc) -> Self {
        Self {
            inner,
            alive_allocs: RefCell::new(Vec::new()),
        }
    }
}

fn assert_disjoint(a: Slice, b: Slice) {
    assert!(
        !((a.ptr >= b.ptr && a.ptr < b.ptr + b.len) || (b.ptr >= a.ptr && b.ptr < a.ptr + a.len))
    );
}

fn check_layout(slice: NonNull<[u8]>, layout: Layout) {
    assert_eq!(slice.cast::<u8>().align_offset(layout.align()), 0);
    assert_eq!(slice.len(), layout.size());
}

unsafe impl<Alloc: Allocator> Allocator for ValidatingAllocator<Alloc> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let x = self.inner.allocate(layout)?;

        if layout.size() > 0 {
            check_layout(x, layout);
            let slice = Slice {
                ptr: x.cast::<u8>().as_ptr() as usize,
                len: x.len(),
            };
            let mut alive_allocs = self.alive_allocs.borrow_mut();
            for other in alive_allocs.iter() {
                assert_disjoint(slice, *other);
            }
            alive_allocs.push(slice);
        }
        Ok(x)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        self.inner.deallocate(ptr, layout);

        if layout.size() > 0 {
            let slice = Slice {
                ptr: ptr.as_ptr() as usize,
                len: layout.size(),
            };
            let mut alive_allocs = self.alive_allocs.borrow_mut();
            for i in 0..alive_allocs.len() {
                if alive_allocs[i] == slice {
                    alive_allocs.swap_remove(i);
                    return;
                }
            }
            panic!("invalid dealloc, slice not found in alive allocation list");
        }
    }

    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        let x = self.inner.grow(ptr, old_layout, new_layout)?;

        if new_layout.size() > 0 {
            let old_slice = Slice {
                ptr: ptr.as_ptr() as usize,
                len: old_layout.size(),
            };
            let new_slice = Slice {
                ptr: x.cast::<u8>().as_ptr() as usize,
                len: x.len(),
            };
            check_layout(x, new_layout);

            let mut alive_allocs = self.alive_allocs.borrow_mut();
            if old_layout.size() > 0 {
                for alive_alloc in alive_allocs.iter_mut() {
                    if alive_alloc == &old_slice {
                        *alive_alloc = new_slice;
                        return Ok(x);
                    }
                }
            } else {
                alive_allocs.push(new_slice);
                return Ok(x);
            }
            panic!("bad grow call");
        }

        Ok(x)
    }

    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        let x = self.inner.shrink(ptr, old_layout, new_layout)?;

        if new_layout.size() > 0 {
            let old_slice = Slice {
                ptr: ptr.as_ptr() as usize,
                len: old_layout.size(),
            };
            let new_slice = Slice {
                ptr: x.cast::<u8>().as_ptr() as usize,
                len: x.len(),
            };
            check_layout(x, new_layout);

            let mut alive_allocs = self.alive_allocs.borrow_mut();
            if old_layout.size() > 0 {
                for alive_alloc in alive_allocs.iter_mut() {
                    if alive_alloc == &old_slice {
                        *alive_alloc = new_slice;
                        return Ok(x);
                    }
                }
            } else {
                alive_allocs.push(new_slice);
                return Ok(x);
            }
            panic!("bad shrink call");
        }
        Ok(x)
    }
}
