
//use crate::consts::*;
use std::thread::{JoinHandle};
use crate::*;
use std::error::Error;

evtcall_error_class!{EvtThreadError}


#[cfg(target_os = "windows")]
include!("thread_windows.rs");

#[cfg(target_os = "linux")]
include!("thread_linux.rs");

#[allow(dead_code)]
pub struct EvtThread<F,T> 
	where 
	T : Send + 'static,
	T : Sync + 'static,
	F : FnOnce() -> T,
	F : Send + 'static,
	F : Sync + 'static, {
	chld : Option<JoinHandle<T>>,
	callfn : F,
	evts : ThreadEvent,
	started : bool,
}


impl<F,T> EvtThread<F,T>
	where 
	T : Send + 'static,
	T : Sync + 'static,
	F : FnOnce() -> T,
	F : Send + 'static,
	F : Sync + 'static,	 {
	pub fn new(callfn :F) -> Result<Self,Box<dyn Error>> {
		let retv : Self = Self {
			chld : None,
			callfn : callfn,
			evts : ThreadEvent::new()?,
			started : false,
		};
		Ok(retv)
	}

	pub fn start(&mut self) -> Result<(),Box<dyn Error>> {
		if !self.started {
			let o = std::thread::spawn(|| {
				let retv = (self.callfn)();
				self.evts.set_exited();
				retv
			});
			self.chld = Some(o);
			self.started = true;
		}
		Ok(())
	}
}