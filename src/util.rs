pub fn align_offset(ptr: usize, align: usize) -> usize {
    align_up(ptr, align) - ptr
}

pub fn align_up(ptr: usize, align: usize) -> usize {
    (ptr + align - 1) & !(align - 1)
}
