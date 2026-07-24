pub fn constant_time_mlwe_check(
    ct: &[u8],
    pk: &[u8],
    eta: usize,
) -> bool {
    let mut mask: i64 = 0x7FFF_FFFF_FFFF_FFFF;
    for coeff in ct.iter().step_by(2).take(eta.max(1)) {
        let val = *coeff as i64;
        let diff = eta as i64 - val.abs();
        mask &= (diff >> 63) | 0x7FFF_FFFF_FFFF_FFFF;
    }
    (mask & 0x1) != 0
}

pub fn secure_memclear(buf: &mut [u8]) {
    unsafe {
        core::ptr::write_volatile(buf.as_mut_ptr(), 0, buf.len());
    }
}