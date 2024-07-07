
use std::sync::RwLock;

#[derive(Clone)]
pub struct EvtChannel<T : std::marker::Send + 'static > {
	inner : Arc<RwLock<EvtChannelInner<T>>>,
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
			inner : Arc::new(RwLock::new(EvtChannelInner::new(maxsize,s)?)),
		};
		Ok(retv)
	}

	pub fn put(&self,bv :T) -> Result<(),Box<dyn Error>> {
		let bres = self.inner.write();
		if bres.is_err() {
			evtcall_new_error!{EvtChannelError,"read error {:?}",bres.err().unwrap()}
		}
		let b = bres.unwrap();
		let retv = b.put(bv);
		return retv;
	}

	pub fn get(&self) -> Result<Option<T>,Box<dyn Error>> {
		let bres = self.inner.write();
		if bres.is_err() {
			evtcall_new_error!{EvtChannelError,"read error {:?}",bres.err().unwrap()}
		}
		let b = bres.unwrap();
		return b.get();
	}

	pub fn get_event(&self) -> u64 {
		let b = self.inner.read().unwrap();
		return b.get_event();
	}

	pub fn reset_event(&self)  -> Result<(),Box<dyn Error>> {
		let bres = self.inner.write();
		if bres.is_err() {
			evtcall_new_error!{EvtChannelError,"read error {:?}",bres.err().unwrap()}
		}
		let b = bres.unwrap();
		return b.reset_event();
	}

	pub fn set_event(&self)   -> Result<(),Box<dyn Error>> {
		let bres = self.inner.write();
		if bres.is_err() {
			evtcall_new_error!{EvtChannelError,"read error {:?}",bres.err().unwrap()}
		}
		let b = bres.unwrap();
		return b.set_event();
	}

	pub fn get_name(&self) -> String {
		let b = self.inner.read().unwrap();
		return b.get_name();
	}

}
