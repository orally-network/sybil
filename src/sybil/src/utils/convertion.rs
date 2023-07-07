#[inline(always)]
pub fn u64_to_i64(num: u64) -> i64 {
    if num < i64::MAX as u64 {
        num as i64
    } else {
        i64::MAX
    }
}