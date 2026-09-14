use crate::types::bkb_pack;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_pack_free(pack: *mut bkb_pack) {
	unsafe { bkb_pack::free(pack) };
}

#[cfg(test)]
mod tests;
