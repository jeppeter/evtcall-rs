

use crate::*;
#[allow(unused_imports)]
use crate::logger::*;
use crate::consts::*;


use std::sync::{Arc,RwLock};
use std::error::Error;


struct EventFdInner {
	evt : i32,
	name :String,
	flags : u32,
}

impl Drop for EventFdInner {
	fn drop(&mut self) {
		self.close();
	}
}

evtcall_error_class!{EventFdError}

impl EventFdInner {
	fn new(initval :i32,flags :u32,name :&str) -> Result<Arc<RwLock<Self>>,Box<dyn Error>> {
		let mut retv :Self = Self {
			evt : -1,
			name : format!("{}",name),
			flags : flags,
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


	#[cfg(feature="debug_mode")]
	pub fn debug_self(&self,_fname :&str,_line :u32) {
		evtcall_log_trace!("[{}:{}]EventFdInner [{}] [{:p}]",_fname,_line,self.name,self);
	}

	#[cfg(not(feature="debug_mode"))]
	pub fn debug_self(&self,_fname :&str,_line :u32) {
	}

	pub fn close(&mut self) {
		self.debug_self(file!(),line!());
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
		if self.evt  < 0 {
			evtcall_new_error!{EventFdError,"{} not valid",self.name}
		}
		let bval = wait_event_fd_timeout_inner(self.evt as u64,0);
		if bval &&  (self.flags & EVENT_NO_AUTO_RESET) == 0 {
			let _ = self.reset_event();
		}
		Ok(bval)
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

	fn wait_event(&self,mills :i32) -> bool {
		return wait_event_fd_timeout_inner(self.evt as u64,mills);
	}

	fn reset_event(&self) -> Result<(),Box<dyn Error>> {
		let reti :i32;
		let mut val : libc::eventfd_t = 1;
		unsafe {
			let _ptr = &mut val;
			reti = libc::eventfd_read(self.evt,_ptr);
		}
		if reti < 0 {
			let erri = get_errno!();
			evtcall_new_error!{EventFdError,"can not eventfd_read {} error {}",self.name,erri}
		}
		Ok(())
	}
}

pub (crate) fn wait_event_fd_timeout_inner(evtfd :u64, mills :i32) -> bool {
	let mut fset :libc::fd_set = unsafe{std::mem::zeroed()};
	let mut tval :libc::timeval = unsafe {std::mem::zeroed()};
	unsafe {
		let _ptr = &mut fset as *mut libc::fd_set;
		libc::FD_ZERO(_ptr);
		libc::FD_SET(evtfd as libc::c_int, _ptr);
	}
	let maxfd = evtfd as i32 + 1;
	let reti : libc::c_int;
	if mills < 0 {
		tval.tv_sec = 0;
		tval.tv_usec = 0;
	} else {
		tval.tv_sec = (mills / 1000) as libc::time_t;
		tval.tv_usec = ((mills % 1000) * 1000) as libc::suseconds_t;
	}
	unsafe {
		let _ptr = &mut fset as *mut libc::fd_set;
		let _nullptr = std::ptr::null_mut::<libc::fd_set>();
		let _tvptr = &mut tval as *mut libc::timeval;
		reti = libc::select(maxfd,_ptr,_nullptr,_nullptr,_tvptr);
	}
	if reti > 0 {
		return true;
	}
	return false;
}
