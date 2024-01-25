
use winapi::um::winnt::{HANDLE};
use std::sync::Arc;
use std::cell::RefCell;


struct EventFdInner {
	evt : HANDLE,
}

impl Drop for EventFdInner {
	fn drop(&mut self) {
		self.close();
	}
}

impl EventFdInner {
	fn new(_initval :i32) -> Result<Self,Box<dyn Error>> {
		let retv :Self = Self {
			evt : NULL_HANDLE_VALUE,
		};
		unsafe {
			retv.evt = CreateEvent	
		}
		
	}
}

pub struct EventFd {
	inner : Arc<RefCell<EventFdInner>>,
}

