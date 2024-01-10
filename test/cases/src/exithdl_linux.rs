
use std::error::Error;

pub fn init_exit_handle() -> Result<u64,Box<dyn Error>> {
	return Ok(0);
}

pub fn fini_exit_handle() {
	return;
}
