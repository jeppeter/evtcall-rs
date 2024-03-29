
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
	pub fn new(_initval :i32,name :&str) -> Result<Self,Box<dyn Error>> {
		let retv :Self = Self {
			inner : EventFdInner::new(_initval,name)?,
		};
		Ok(retv)
	}

	pub fn close(&mut self) {		
		evtcall_log_trace!("close EventFd");
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

}

unsafe impl Sync for EventFd {}
unsafe impl Send for EventFd {}