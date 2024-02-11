
#[cfg(target_os = "windows")]
include!("channel_windows.rs");

#[cfg(target_os = "linux")]
include!("channel_linux.rs");

#[derive(Clone)]
pub struct EvtChannel<T : std::marker::Send + 'static > {
	inner : Arc<RefCell<EvtChannelInner<T>>>,
}

impl<T : std::marker::Send + 'static > Drop for EvtChannel<T> {
	fn drop(&mut self) {
		self.close();
	}
}

impl<T : std::marker::Send + 'static > EvtChannel<T> {
	pub fn close(&mut self) {
		evtcall_log_trace!("close EvtChannel");
	}

	pub fn new(maxsize :usize, s :&str) -> Result<Self, Box<dyn Error>> {
		let retv :Self = Self {
			inner : EvtChannelInner::new(maxsize,s)?,
		};
		Ok(retv)
	}

	pub fn put(&self,bv :T) -> Result<(),Box<dyn Error>> {
		return self.inner.borrow().put(bv);
	}

	pub fn get(&self) -> Result<Option<T>,Box<dyn Error>> {
		return self.inner.borrow().get();
	}

	pub fn get_evt(&self) -> u64 {
		return self.inner.borrow().get_evt();
	}

	pub fn reset_evt(&self)  -> Result<(),Box<dyn Error>> {
		return self.inner.borrow().reset_evt();
	}

	pub fn set_evt(&self)   -> Result<(),Box<dyn Error>> {
		return self.inner.borrow().set_evt();
	}

}