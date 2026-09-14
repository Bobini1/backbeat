use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::error::{BKB_ERR_PANIC, BKB_OK, bkb_error_code};

pub(crate) fn run_ffi(op: impl FnOnce() -> Result<(), bkb_error_code>) -> bkb_error_code {
	match catch_unwind(AssertUnwindSafe(op)) {
		Ok(Ok(())) => BKB_OK,
		Ok(Err(code)) => code,
		Err(_) => BKB_ERR_PANIC,
	}
}
