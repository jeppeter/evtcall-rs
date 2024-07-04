
#[cfg(target_os = "windows")]
include!("channel_windows.rs");

#[cfg(target_os = "linux")]
include!("channel_linux.rs");


#[derive(Clone)]
pub struct EvtChannel<T : std::marker::Send + 'static > {
	inner : Arc<UnsafeCell<EvtChannelInner<T>>>,
}

impl<T : std::marker::Send + 'static > Drop for EvtChannel<T> {
	fn drop(&mut self) {
		self.close();
	}
}

impl<T : std::marker::Send + 'static > EvtChannel<T> {
	pub fn close(&mut self) {
		evtcall_log_trace!("close EvtChannel [{}]",self.get_name());
	}

	pub fn new(maxsize :usize, s :&str) -> Result<Self, Box<dyn Error>> {
		let retv :Self = Self {
			inner : EvtChannelInner::new(maxsize,s)?,
		};
		Ok(retv)
	}

	pub fn put(&self,bv :T) -> Result<(),Box<dyn Error>> {
		let s1 = unsafe {&*self.inner.get()};
		return s1.put(bv);
	}

	pub fn get(&self) -> Result<Option<T>,Box<dyn Error>> {
		let s1 = unsafe {&*self.inner.get()};
		return s1.get();
	}

	pub fn get_event(&self) -> u64 {
		let s1 = unsafe {&*self.inner.get()};
		return s1.get_event();
	}

	pub fn reset_event(&self)  -> Result<(),Box<dyn Error>> {
		let s1 = unsafe {&*self.inner.get()};
		return s1.reset_event();
	}

	pub fn set_event(&self)   -> Result<(),Box<dyn Error>> {
		let s1 = unsafe {&*self.inner.get()};
		return s1.set_event();
	}

	pub fn get_name(&self) -> String {
		let s1 = unsafe {&*self.inner.get()};
		return s1.get_name();
	}
}

unsafe impl<T : std::marker::Send + 'static > Sync for EvtChannel<T> {}
unsafe impl<T : std::marker::Send + 'static > Send for EvtChannel<T> {}