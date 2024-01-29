

use crate::*;
use crate::logger::*;
use crate::consts::*;


use std::sync::{Arc,RwLock};
use std::error::Error;


struct EventFdInner {
	evt : i32,
	name :String,
}

impl Drop for EventFdInner {
	fn drop(&mut self) {
		self.close();
	}
}

evtcall_error_class!{EventFdError}

impl EventFdInner {
	//fn new(_initval :i32,name :&str) -> Result<Arc<RefCell<Self>>,Box<dyn Error>> {
	fn new(initval :i32,name :&str) -> Result<Arc<RwLock<Self>>,Box<dyn Error>> {
		let mut retv :Self = Self {
			evt : -1,
			name : format!("{}",name),
		};
		let flags :libc::c_int = libc::EFD_NONBLOCK;
		unsafe {
			retv.evt = libc::eventfd(initval as u32,flags);
		}
		if retv.evt < 0 {
			let erri = get_errno!();
			evtcall_new_error!{EventFdError,"can not init {} error {}",name,erri}
		}
		Ok(Arc::new(RwLock::new(retv)))
	}

	pub fn close(&mut self) {
		evtcall_log_trace!("close EventFdInner");
		if self.evt >= 0 {
			unsafe {
				libc::close(self.evt);
			}
			self.evt = -1;
		}
	}

	fn set_event(&self) -> Result<(),Box<dyn Error>> {
		let mut reti :i32;
		let val : libc::eventfd_t = 1;
		if self.evt < 0 {
			evtcall_new_error!{EventFdError,"{} not set event",self.name}
		}
		unsafe {
			reti = libc::eventfd_write(self.evt,val);
		}
		if reti < 0 {
			reti = get_errno!();
			evtcall_new_error!{EventFdError,"can not set event {} error {}",self.name,reti}
		}
		Ok(())
	}

	fn is_event(&self) -> Result<bool,Box<dyn Error>> {
		let mut reti :i32;
		if self.evt  < 0 {
			evtcall_new_error!{EventFdError,"{} not valid",self.name}
		}
		let mut val :libc::eventfd_t = 0;
		unsafe {
			let _ptr = &mut val;
			reti = libc::eventfd_read(self.evt,_ptr);
		}
		if reti < 0 {
			reti = get_errno!();
			if reti == -libc::EAGAIN || reti == -libc::EWOULDBLOCK {
				return Ok(false);
			}
			evtcall_new_error!{EventFdError,"{} read error {}",self.name,reti}
		} 
		if val > 0 {
			return Ok(true);
		}
		return Ok(false);
	}

	fn get_event(&self) -> u64 {
		let mut retv :u64 = INVALID_EVENT_HANDLE;
		if self.evt >= 0 {
			retv = self.evt as u64;
		}
		return retv;
	}

	fn get_name(&self) -> String {
		return format!("{}",self.name);
	}
}

