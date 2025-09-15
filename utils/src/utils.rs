use super::error::{BrineTreeError, ProgramResult};

#[inline]
/// Check a condition and return a custom error if false.
pub fn check_condition(condition: bool, err: BrineTreeError) -> ProgramResult {
    if condition {
        Ok(())
    } else {
        Err(err)
    }
}
