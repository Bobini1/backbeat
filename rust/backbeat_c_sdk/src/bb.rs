use std::os::raw::c_char;

use backbeat_core::BackbeatFile;

use crate::error::{BKB_ERR_NOT_FOUND, bkb_error_code, run_backbeat_file};
use crate::types::{bkb_bb, bkb_bytes, bkb_string, bytes_from_raw, cstr_to_str};
use crate::util::run_ffi;

unsafe fn with_bb<T>(
	bb: *const bkb_bb,
	op: impl FnOnce(&BackbeatFile) -> T,
) -> Result<T, bkb_error_code> {
	if bb.is_null() {
		return Err(crate::error::BKB_ERR_NULL_ARG);
	}
	Ok(op(unsafe { (*bb).inner() }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_bb_from_file(
	path: *const c_char,
	out_bb: *mut *mut bkb_bb,
) -> bkb_error_code {
	run_ffi(|| {
		if out_bb.is_null() {
			return Err(crate::error::BKB_ERR_NULL_ARG);
		}
		let path = unsafe { cstr_to_str(path) }?;
		let inner = run_backbeat_file(|| BackbeatFile::from_file(path))?;
		let bb = bkb_bb::alloc(inner);
		unsafe { out_bb.write(bb) };
		Ok(())
	})
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_bb_from_json(
	json: *const u8,
	json_len: usize,
	out_bb: *mut *mut bkb_bb,
) -> bkb_error_code {
	run_ffi(|| {
		if out_bb.is_null() {
			return Err(crate::error::BKB_ERR_NULL_ARG);
		}
		let json = unsafe { bytes_from_raw(json, json_len) }?;
		let inner = run_backbeat_file(|| BackbeatFile::from_json(json))?;
		let bb = bkb_bb::alloc(inner);
		unsafe { out_bb.write(bb) };
		Ok(())
	})
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_bb_free(bb: *mut bkb_bb) {
	unsafe { bkb_bb::free(bb) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_bb_to_json(
	bb: *const bkb_bb,
	out_json: *mut bkb_bytes,
) -> bkb_error_code {
	run_ffi(|| {
		if out_json.is_null() {
			return Err(crate::error::BKB_ERR_NULL_ARG);
		}
		let json = unsafe { with_bb(bb, BackbeatFile::to_json) }?;
		unsafe { bkb_bytes::write(out_json, json) }
	})
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_bb_chart_sha256(
	bb: *const bkb_bb,
	out_sha256: *mut bkb_string,
) -> bkb_error_code {
	run_ffi(|| {
		let sha256 = unsafe { with_bb(bb, BackbeatFile::chart_sha256) }?;
		unsafe { bkb_string::write(out_sha256, sha256.to_string()) }
	})
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_bb_bundle_id(
	bb: *const bkb_bb,
	out_bundle_id: *mut bkb_string,
) -> bkb_error_code {
	run_ffi(|| {
		let bundle_id = unsafe { with_bb(bb, BackbeatFile::bundle_id) }?;
		unsafe { bkb_string::write(out_bundle_id, bundle_id.to_string()) }
	})
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_bb_combined_assets_id(
	bb: *const bkb_bb,
	out_combined_assets_id: *mut bkb_string,
) -> bkb_error_code {
	run_ffi(|| {
		let combined_assets_id = unsafe { with_bb(bb, BackbeatFile::combined_assets_id) }?;
		unsafe { bkb_string::write(out_combined_assets_id, combined_assets_id.to_string()) }
	})
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_bb_resolve_path(
	bb: *const bkb_bb,
	path: *const c_char,
	out_asset_id: *mut bkb_string,
) -> bkb_error_code {
	run_ffi(|| {
		let path = unsafe { cstr_to_str(path) }?;
		let asset_id =
			unsafe { with_bb(bb, |bb| bb.resolve_path(path)) }?.ok_or(BKB_ERR_NOT_FOUND)?;
		unsafe { bkb_string::write(out_asset_id, asset_id.to_string()) }
	})
}

#[cfg(test)]
mod tests;
