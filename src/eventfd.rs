
#[cfg(target_os = "windows")]
include!("eventfd_windows.rs");

#[cfg(target_os = "linux")]
include!("eventfd_linux.rs");

#[derive(Clone)]
pub struct EventFd {
	inner : Arc<RwLock<EventFdInner>>,
}

impl Drop for EventFd {
	fn drop(&mut self) {
		self.close();
	}
}


impl EventFd {
	pub fn new(_initval :i32, flags :u32,name :&str) -> Result<Self,Box<dyn Error>> {
		let retv :Self = Self {
			inner : EventFdInner::new(_initval,flags,name)?,
		};
		Ok(retv)
	}

	pub fn debug_self(&self,_fname :&str,_line :u32) {
		let _name :String;
		let _cnt :usize;
		{
			let cv = self.inner.read().unwrap();
			_name = cv.get_name();
			_cnt = Arc::strong_count(&self.inner);
		}		
		evtcall_log_trace!("[{}:{}]EventFd [{}] cnt [{}] [{:p}]",_fname,_line,_name,_cnt,self);
	}

	pub fn close(&mut self) {
		self.debug_self(file!(),line!());
		evtcall_log_trace!("close EventFd [{:p}]",self);
	}

	pub fn is_event(&self) -> Result<bool,Box<dyn Error>> {
		let bres = self.inner.read();
		if bres.is_err() {
			evtcall_new_error!{EventFdError,"read error"}
		}
		let b = bres.unwrap();
		let retv = b.is_event();
		return retv;
	}

	pub fn set_event(&self) -> Result<(),Box<dyn Error>> {
		let bres = self.inner.read();
		if bres.is_err() {
			evtcall_new_error!{EventFdError,"read error"}
		}
		let b = bres.unwrap();
		let retv = b.set_event();
		return retv;
	}

	pub fn get_event(&self) -> u64 {
		let bres = self.inner.read();
		if bres.is_err() {
			return INVALID_EVENT_HANDLE;
		}
		let b = bres.unwrap();
		let retv = b.get_event();
		return retv;
	}

	pub fn get_name(&self) -> String {
		let bres = self.inner.read();
		if bres.is_err() {
			return format!("");
		}
		let b = bres.unwrap();
		let retv = b.get_name();
		return retv;
	}

	pub fn wait_event(&self,mills :i32) -> bool {
		let bres = self.inner.read();
		if bres.is_err() {
			return false;
		}
		let b = bres.unwrap();
		return b.wait_event(mills);
	}

	pub fn reset_event(&self) -> Result<(),Box<dyn Error>> {
		let bres = self.inner.read();
		if bres.is_err() {
			let no = bres.err();
			return Err(no).unwrap();
		}
		let b = bres.unwrap();
		return b.reset_event();		
	}

}

pub fn wait_event_fd_timeout(evtfd :u64, mills :i32) -> bool {
	wait_event_fd_timeout_inner(evtfd,mills)
}


unsafe impl Sync for EventFd {}
unsafe impl Send for EventFd {}