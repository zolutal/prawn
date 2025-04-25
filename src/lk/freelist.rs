//! helpers for dealing with CONFIG_HARDENED_FREELIST

/// Does a 64-bit swab (swap bytes) operation
///
/// Args:
/// * `val` - value to swab
///
/// Return:
/// * Byte order reversed `val`
pub fn swab(val: u64) -> u64 {
    return val.swap_bytes();
}

/// Deobfuscate a leaked freelist pointer target, when address & cookie are known
///
/// Args:
/// * `leaked` - The leaked obfuscated freelist pointer
/// * `slab_cookie` - The random value associated with the slab
/// * `addr` - The address of the leaked freelist pointer
///
/// Return:
/// * The decoded target of the freelist pointer
pub fn deob_freelist_ptr_target(leaked: u64, slab_cookie: u64, addr: u64) -> u64 {
    leaked ^ slab_cookie ^ swab(addr)
}

/// Deobfuscate a leaked freelist pointer address, when target & cookie are known
///
/// Args:
///
/// * `leaked` - The leaked obfuscated freelist pointer
/// * `slab_cookie` - The random value associated with the slab
/// * `target` - The address pointed to by the leaked freelist pointer
///
/// Return:
/// * The decoded address of the freelist pointer
pub fn deob_freelist_ptr_addr(leaked: u64, slab_cookie: u64, target: u64) -> u64 {
    swab(leaked ^ slab_cookie ^ target)
}

/// Calculate a slab cookie, when target & address of a freelist pointer are known
///
/// Args:
/// * `leaked` - The leaked obfuscated freelist pointer
/// * `addr` - The address of the leaked freelist pointer
/// * `target` - The address pointed to by the leaked freelist pointer
///
/// Return:
/// * The random value used to obfuscate the freelist pointer
pub fn calc_freelist_cookie(leaked: u64, addr: u64, target: u64) -> u64 {
    leaked ^ swab(addr) ^ target
}
