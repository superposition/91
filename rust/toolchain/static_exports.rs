// SPDX-License-Identifier: GPL-3.0-only
//! Static symbol spelling and readonly/interior-mutability compiler regression.
use core::cell::UnsafeCell;
#[export_name = "fm1_export_value"]
pub static EXPORT_VALUE: u32 = 0x1122_3344;
#[no_mangle]
pub static FM1_NOMANGLE_VALUE: u32 = 0x5566_7788;
#[repr(transparent)]
pub struct ExportCell(UnsafeCell<u32>);
// SAFETY: the only mutation is through the exclusive-access probe below.
unsafe impl Sync for ExportCell {}
#[export_name = "fm1_export_cell"]
pub static EXPORT_CELL: ExportCell = ExportCell(UnsafeCell::new(17));
/// # Safety
/// Exclude every other access to the exported probe cell during this call.
#[no_mangle]
pub unsafe extern "C" fn fm1_export_cell_step() -> u32 {
    // SAFETY: the caller supplies exclusive access; the cell is live and aligned.
    unsafe {
        let p = EXPORT_CELL.0.get();
        let next = (*p).wrapping_add(1);
        *p = next;
        next
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exported_values_and_interior_mutability_match_rust() {
        assert_eq!(EXPORT_VALUE, 0x1122_3344);
        assert_eq!(FM1_NOMANGLE_VALUE, 0x5566_7788);
        // SAFETY: this is the only test accessing this independent probe cell.
        assert_eq!(unsafe { fm1_export_cell_step() }, 18);
    }
}
