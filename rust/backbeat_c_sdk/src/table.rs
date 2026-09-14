use crate::error::{BKB_ERR_NULL_ARG, bkb_error_code};
use crate::types::bkb_table;
use crate::util::run_ffi;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_table_free(table: *mut bkb_table) {
	unsafe { bkb_table::free(table) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_table_chart_count(
	table: *const bkb_table,
	out_count: *mut usize,
) -> bkb_error_code {
	run_ffi(|| {
		if table.is_null() || out_count.is_null() {
			return Err(BKB_ERR_NULL_ARG);
		}
		let table = unsafe { &*table };
		let count = if table.levels_len == 0 {
			0
		} else {
			unsafe { std::slice::from_raw_parts(table.levels, table.levels_len) }
				.iter()
				.map(|level| level.charts_len)
				.sum()
		};
		unsafe { out_count.write(count) };
		Ok(())
	})
}

#[cfg(test)]
mod tests;
