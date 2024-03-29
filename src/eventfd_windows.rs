
use winapi::um::winnt::{HANDLE,LPCWSTR};
use winapi::um::minwinbase::{LPSECURITY_ATTRIBUTES,SECURITY_ATTRIBUTES};
use winapi::um::synchapi::*;
use winapi::shared::minwindef::{TRUE,FALSE,DWORD,BOOL};
use winapi::um::errhandlingapi::{GetLastError};
use winapi::um::handleapi::{CloseHandle};
use winapi::um::winbase::{WAIT_OBJECT_0};
use winapi::shared::winerror::{WAIT_TIMEOUT};
use std::sync::{Arc,RwLock};
use std::error::Error;

use crate::*;
use crate::logger::*;
use crate::consts_windows::*;
use crate::consts::*;


struct EventFdInner {
	evt : HANDLE,
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
	fn new(_initval :i32,name :&str) -> Result<Arc<RwLock<Self>>,Box<dyn Error>> {
		let mut retv :Self = Self {
			evt : NULL_HANDLE_VALUE,
			name : format!("{}",name),
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

	pub fn close(&mut self) {
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
		let dret :DWORD;
		if self.evt == NULL_HANDLE_VALUE {
			evtcall_new_error!{EventFdError,"{} not valid",self.name}
		}
		unsafe {
			dret = WaitForSingleObject(self.evt,0);
		}
		if dret == WAIT_OBJECT_0 {
			unsafe {
				ResetEvent(self.evt);
			}
			return Ok(true);
		} else if dret == WAIT_TIMEOUT {
			return Ok(false);
		}
		let erri = get_errno!();
		evtcall_new_error!{EventFdError,"get event {} error {}",self.name,erri}
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
}


