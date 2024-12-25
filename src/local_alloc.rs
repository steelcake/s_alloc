use core::alloc::{AllocError, Allocator, Layout};
use core::ptr::NonNull;
use std::cell::RefCell;
use std::ops::DerefMut;

use crate::page_alloc::PageAlloc;
use crate::util::{align_offset, align_up};

// Use this to avoid creating aliased pointers.
// Not sure of all the details of unsafety of that but doing it to make "sure".
type Ptr = usize;

#[derive(Clone, Copy)]
struct Slice {
    ptr: Ptr,
    len: usize,
}

struct InnerLocalAlloc<'a> {
    page_alloc: &'a dyn PageAlloc,
    pages: Vec<Slice>,
    free_list: Vec<Vec<Slice>>,
    free_after: usize,
    error_after: usize,
    min_page_size: usize,
    total_page_size: usize,
    ptr_to_size: Vec<(usize, usize)>,
}

pub struct LocalAlloc<'a> {
    inner: RefCell<InnerLocalAlloc<'a>>,
}

impl Drop for LocalAlloc<'_> {
    fn drop(&mut self) {
        let this = self.inner.borrow_mut();
        for page in this.pages.iter() {
            // Safety: there are no references to the pages remaning at the time of drop.
            // So constructing a pointer to the page itself doesn't alias. Pages are never null.
            // And we only allocate pages using self.page_alloc
            unsafe {
                let ptr = NonNull::new(page.ptr as *mut u8).unwrap();
                let len = page.len;

                let page = NonNull::slice_from_raw_parts(ptr, len);

                this.page_alloc.dealloc_page(page);
            }
        }
    }
}

pub struct Config<'a> {
    page_alloc: &'a dyn PageAlloc,
    free_after: usize,
    error_after: usize,
    min_page_size: usize,
}

impl<'a> Config<'a> {
    pub fn new(page_alloc: &'a dyn PageAlloc) -> Self {
        Self {
            page_alloc,
            free_after: 1 << 29, // 512 MB
            error_after: usize::MAX,
            min_page_size: 1 << 27, // 128 MB
        }
    }

    pub fn free_after(&mut self, free_after: usize) -> &mut Self {
        self.free_after = free_after;
        self
    }

    pub fn error_after(&mut self, error_after: usize) -> &mut Self {
        self.error_after = error_after;
        self
    }

    pub fn min_page_size(&mut self, min_page_size: usize) -> &mut Self {
        self.min_page_size = min_page_size;
        self
    }
}

impl<'a> LocalAlloc<'a> {
    pub fn new(config: Config<'a>) -> Self {
        Self {
            inner: RefCell::new(InnerLocalAlloc {
                page_alloc: config.page_alloc,
                free_after: config.free_after,
                error_after: config.error_after,
                min_page_size: config.min_page_size,
                free_list: Vec::new(),
                pages: Vec::new(),
                total_page_size: 0,
                ptr_to_size: Vec::new(),
            }),
        }
    }

    fn try_alloc_in_existing_pages(
        this: &mut InnerLocalAlloc,
        layout: Layout,
    ) -> Option<NonNull<[u8]>> {
        // Try to find a page that fits this allocation
        for free_ranges in this.free_list.iter_mut() {
            for free_range_idx in 0..free_ranges.len() {
                let free_range = *free_ranges.get(free_range_idx).unwrap();
                let alignment_offset = align_up(free_range.ptr, layout.align());
                let needed_size = alignment_offset + layout.size();
                if free_range.len >= needed_size {
                    if alignment_offset > 0 {
                        free_ranges.push(Slice {
                            ptr: free_range.ptr,
                            len: alignment_offset,
                        });
                    }
                    if free_range.len > needed_size {
                        free_ranges.push(Slice {
                            ptr: free_range.ptr + needed_size,
                            len: free_range.len - needed_size,
                        })
                    }
                    free_ranges.swap_remove(free_range_idx);

                    let ptr = NonNull::new((free_range.ptr + alignment_offset) as *mut u8).unwrap();
                    return Some(NonNull::slice_from_raw_parts(ptr, layout.size()));
                }
            }
        }

        None
    }

    fn alloc_in_new_page(this: &mut InnerLocalAlloc, page: Slice, layout: Layout) -> NonNull<[u8]> {
        assert_ne!(layout.size(), 0);
        assert!(page.len >= layout.size());
        assert_eq!(align_offset(page.ptr, layout.align()), 0);

        this.pages.push(page);

        let mut free_ranges = Vec::new();
        if layout.size() < page.len {
            free_ranges.push(Slice {
                ptr: page.ptr + layout.size(),
                len: page.len - layout.size(),
            });
        }
        this.free_list.push(free_ranges);

        let ptr = NonNull::new(page.ptr as *mut u8).unwrap();
        NonNull::slice_from_raw_parts(ptr, layout.size())
    }

    fn free_pages_if_needed(this: &mut InnerLocalAlloc) {
        if this.free_after >= this.total_page_size {
            return;
        }

        let mut page_index = 0;

        // try to find any empty pages and free them
        // this loop pattern is used because we need to remove items while iterating
        while page_index < this.pages.len() {
            let free_r = this.free_list.get_mut(page_index).unwrap();
            if free_r.len() == 1 {
                let range = *free_r.first().unwrap();
                let page = *this.pages.get(page_index).unwrap();
                if range.ptr == page.ptr && range.len == page.len {
                    this.pages.swap_remove(page_index);
                    this.free_list.swap_remove(page_index);

                    this.total_page_size -= page.len;

                    let ptr = NonNull::new(page.ptr as *mut u8).unwrap();
                    let page = NonNull::slice_from_raw_parts(ptr, page.len);
                    // Safety: we allocate these pages with the same page alloc.
                    // page and it's free_ranges are removed from the data structure immediately
                    // before freeing the page.
                    unsafe { this.page_alloc.dealloc_page(page) };

                    continue;
                }
            }
            page_index += 1;
        }
    }

    fn alloc(this: &mut InnerLocalAlloc, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        if this.error_after <= this.total_page_size {
            return Err(AllocError);
        }

        // Alignment over 4KB is not allowed
        if layout.align() > 1 << 12 {
            return Err(AllocError);
        }

        if layout.size() == 0 {
            return Ok(NonNull::slice_from_raw_parts(NonNull::dangling(), 0));
        }

        if let Some(res) = Self::try_alloc_in_existing_pages(this, layout) {
            this.ptr_to_size
                .push((res.cast::<u8>().as_ptr() as usize, res.len()));
            return Ok(res);
        }

        let page_alloc_size = layout.size().max(this.min_page_size);
        let page = this.page_alloc.alloc_page(page_alloc_size)?;
        let page = Slice {
            ptr: page.cast::<u8>().as_ptr() as usize,
            len: page.len(),
        };
        this.total_page_size += page.len;

        let x = Self::alloc_in_new_page(this, page, layout);
        this.ptr_to_size
            .push((x.cast::<u8>().as_ptr() as usize, x.len()));

        Ok(x)
    }

    fn dealloc(this: &mut InnerLocalAlloc, ptr: NonNull<u8>, size: usize) {
        if size == 0 {
            return;
        }

        let addr = ptr.as_ptr() as usize;
        let size_idx = this
            .ptr_to_size
            .iter()
            .position(|x| x.0 == addr)
            .expect("find allocation index");
        let size = this.ptr_to_size.swap_remove(size_idx).1;

        let start_addr = ptr.as_ptr() as usize;
        let end_addr = start_addr + size;

        for page_idx in 0..this.pages.len() {
            {
                let page = *this.pages.get(page_idx).unwrap();
                let contains = start_addr >= page.ptr && page.ptr + page.len >= end_addr;
                if !contains {
                    continue;
                }
            }

            let free_ranges = this.free_list.get_mut(page_idx).unwrap();
            let mut range_to_insert = Slice {
                ptr: start_addr,
                len: size,
            };
            let mut free_range_idx = 0;
            let mut found = false;
            // Try to find adjacent free ranges to the free range we want to insert.
            // We might have two such ranges, one to the left of our range and one to the right.
            while free_range_idx < free_ranges.len() {
                let free_range = *free_ranges.get(free_range_idx).unwrap();
                if free_range.ptr == end_addr {
                    range_to_insert = Slice {
                        ptr: range_to_insert.ptr,
                        len: range_to_insert.len + free_range.len,
                    };
                    free_ranges.swap_remove(free_range_idx);
                    if found {
                        break;
                    }
                    found = true;
                } else if free_range.ptr + free_range.len == start_addr {
                    range_to_insert = Slice {
                        ptr: free_range.ptr,
                        len: range_to_insert.len + free_range.len,
                    };
                    free_ranges.swap_remove(free_range_idx);
                    if found {
                        break;
                    }
                    found = true;
                } else {
                    free_range_idx += 1;
                }
            }

            free_ranges.push(range_to_insert);

            Self::free_pages_if_needed(this);

            return;
        }

        panic!("bad deallocate");
    }
}

// Safety: pointers given by local alloc point to actual pages and not to inside the struct itself.
// So it is safe to move a LocalAlloc while there are live allocations on it.
unsafe impl Allocator for LocalAlloc<'_> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let mut this = self.inner.borrow_mut();
        let this = this.deref_mut();
        Self::alloc(this, layout)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        let mut this = self.inner.borrow_mut();
        let this = this.deref_mut();
        Self::dealloc(this, ptr, layout.size())
    }

    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        mut old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocError> {
        assert!(new_layout.size() > old_layout.size());
        assert_eq!(old_layout.align(), new_layout.align());

        {
            let mut this = self.inner.borrow_mut();
            let this = this.deref_mut();

            if old_layout.size() == 0 && new_layout.size() > 0 {
                return Self::alloc(this, new_layout);
            }

            if old_layout.size() > 0 {
                let idx = this
                    .ptr_to_size
                    .iter()
                    .position(|x| x.0 == ptr.as_ptr() as usize)
                    .expect("find old alloc size");
                let size = this.ptr_to_size.remove(idx).1;
                old_layout = Layout::from_size_align(size, old_layout.align()).unwrap();
            }

            let end_addr = (ptr.as_ptr() as usize) + old_layout.size();

            'try_alloc: for free_ranges in this.free_list.iter_mut() {
                for free_range_idx in 0..free_ranges.len() {
                    let free_range = *free_ranges.get(free_range_idx).unwrap();
                    if free_range.ptr == end_addr {
                        if free_range.len + old_layout.size() > new_layout.size() {
                            let size_diff = new_layout.size() - old_layout.size();
                            free_ranges[free_range_idx].len -= size_diff;
                            this.ptr_to_size
                                .push((ptr.as_ptr() as usize, new_layout.size()));
                            return Ok(NonNull::slice_from_raw_parts(ptr, new_layout.size()));
                        } else if free_range.len + old_layout.size() == new_layout.size() {
                            free_ranges.swap_remove(free_range_idx);
                            this.ptr_to_size
                                .push((ptr.as_ptr() as usize, new_layout.size()));
                            return Ok(NonNull::slice_from_raw_parts(ptr, new_layout.size()));
                        } else {
                            break 'try_alloc;
                        }
                    }
                }
            }

            if old_layout.size() > 0 {
                this.ptr_to_size
                    .push((ptr.as_ptr() as usize, old_layout.size()));
            }
        } // end "this" scope

        let new_ptr = self.allocate(new_layout)?;

        std::ptr::copy_nonoverlapping(
            ptr.as_ptr(),
            new_ptr.cast::<u8>().as_ptr(),
            old_layout.size(),
        );
        self.deallocate(ptr.cast::<u8>(), old_layout);

        Ok(new_ptr)
    }
}
