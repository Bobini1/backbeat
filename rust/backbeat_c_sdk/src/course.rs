use crate::types::bkb_course;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bkb_course_free(course: *mut bkb_course) {
	unsafe { bkb_course::free(course) };
}

#[cfg(test)]
mod tests;
