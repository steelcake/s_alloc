pub fn align_offset(ptr: usize, align: usize) -> usize {
    align_up(ptr, align) - ptr
}

pub fn align_up(ptr: usize, align: usize) -> usize {
    (ptr + align - 1) & !(align - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align_up_1() {
        for ptr in 0..1024 {
            assert_eq!(align_offset(ptr, 1), 0);
        }
    }
}
