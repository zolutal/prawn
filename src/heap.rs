/// Decrypts a leaked safelinking-protected pointer assuming the next pointer
/// is on the same page
pub fn sl_demangle(ptr: u64) -> u64 {
    let pos = ptr >> 12;
    let middle = pos ^ ptr;
    middle >> 24 ^ middle
}

/// Implements safelinking mangling of a chunk pointer with a target pointer
pub fn sl_mangle(pos: u64, ptr: u64) -> u64 {
    pos >> 12 ^ ptr
}
