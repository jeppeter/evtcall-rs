
use winapi::um::winnt::{HANDLE,LPCWSTR};
use winapi::um::minwinbase::{LPSECURITY_ATTRIBUTES,SECURITY_ATTRIBUTES};
use winapi::um::synchapi::*;
use winapi::shared::minwindef::{TRUE,FALSE,DWORD,BOOL};
use winapi::um::errhandlingapi::{GetLastError};
use winapi::um::handleapi::{CloseHandle};
use winapi::um::winbase::{WAIT_OBJECT_0};
//use winapi::shared::winerror::{WAIT_TIMEOUT};
use std::sync::{Arc,RwLock};
use std::error::Error;

use crate::*;
#[allow(unused_imports)]
use crate::logger::*;
use crate::consts_windows::*;
use crate::consts::*;


struct EventFdInner {
	evt : HANDLE,
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
	//fn new(_initval :i32,name :&str) -> Result<Arc<RefCell<Self>>,Box<dyn Error>> {
	fn new(_initval :i32,flags :u32,name :&str) -> Result<Arc<RwLock<Self>>,Box<dyn Error>> {
		let mut retv :Self = Self {
			evt : NULL_HANDLE_VALUE,
			name : format!("{}",name),
			flags : flags,
		};
		let _errval :i32;
		let _pattr :LPSECURITY_ATTRIBUTES = std::ptr::null_mut::<SECURITY_ATTRIBUTES>() as LPSECURITY_ATTRIBUTES;
		let _pstr :LPCWSTR = std::ptr::null() as LPCWSTR;
		unsafe {
			retv.evt = CreateEventW(_pattr,TRUE,FALSE,_pstr);
		}
		if retv.evt == NULL_HANDLE_VALUE {
			_errval = get_errno!();
			evtcall_new_error!{EventFdError,"can not CreateEventW error {}",_errval}
		}
		Ok(Arc::new(RwLock::new(retv)))
	}

	#[cfg(feature="debug_mode")]
	pub fn debug_self(&self,_fname :&str,_line :u32) {
		evtcall_log_trace!("[{}:{}]EventFdInner [{}] [{:p}]",_fname,_line,self.name,self);
	}

	#[cfg(not(feature="debug_mode"))]
	pub fn debug_self(&self,_fname :&str,_line :u32) {
		return;
	}

	pub fn close(&mut self) {
		self.debug_self(file!(),line!());
		evtcall_log_trace!("close EventFdInner");
		if self.evt != NULL_HANDLE_VALUE {
			unsafe {
				CloseHandle(self.evt);
			}
			self.evt = NULL_HANDLE_VALUE;
		}
	}

	fn set_event(&self) -> Result<(),Box<dyn Error>> {
		let bret :BOOL ;
		unsafe {
			bret = SetEvent(self.evt);
		}
		if bret == FALSE {
			let erri = get_errno!();
			evtcall_new_error!{EventFdError,"can not set event {} error {}",self.name,erri}
		}
		Ok(())
	}

	fn is_event(&self) -> Result<bool,Box<dyn Error>> {
		if self.evt == NULL_HANDLE_VALUE {
			evtcall_new_error!{EventFdError,"{} not valid",self.name}
		}

		let bval = self.wait_event(0);
		if bval && (self.flags & EVENT_NO_AUTO_RESET) == 0 {
			let _ = self.reset_event();
		}
		Ok(bval)
	}

	fn get_event(&self) -> u64 {
		let mut retv :u64 = INVALID_EVENT_HANDLE;
		if self.evt != NULL_HANDLE_VALUE {
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
		unsafe {
			ResetEvent(self.evt);
		}
		Ok(())
	}
}

pub (crate) fn wait_event_fd_timeout_inner(evtfd :u64, mills :i32) -> bool {
	let waithd :HANDLE = evtfd as HANDLE;
	let dret :DWORD;
	if mills < 0 {
		unsafe {
			dret = WaitForSingleObject(waithd,0);	
		}		
	} else {
		unsafe {
			dret = WaitForSingleObject(waithd,mills as u32);	
		}		
	}

	if dret == WAIT_OBJECT_0 {
		return true;
	}
	return false;
}

