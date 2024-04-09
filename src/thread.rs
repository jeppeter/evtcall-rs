
//use crate::consts::*;
use std::thread::{JoinHandle};
use crate::*;
use crate::logger::*;
use std::error::Error;
use std::cell::UnsafeCell;
use std::sync::{Arc,RwLock};
use crate::eventfd::*;
evtcall_error_class!{EvtThreadError}


struct ThreadEventInner {
	exitevt : EventFd,
	noteevt : EventFd,
}

impl ThreadEventInner {
	pub (crate) fn new() -> Result<Self,Box<dyn Error>> {
		let retv :Self = Self {
			exitevt : EventFd::new(0,"exit event")?,
			noteevt : EventFd::new(0,"notice event")?,
		};
		Ok(retv)
	}

	pub (crate) fn set_exit_event(&mut self) -> Result<(),Box<dyn Error>> {
		self.exitevt.set_event()
	}

	pub (crate) fn set_notice_exit_event(&mut self) -> Result<(),Box<dyn Error>> {
		self.noteevt.set_event()
	}

	pub (crate) fn get_exit_event(&self) -> u64 {
		self.exitevt.get_event()
	}

	pub (crate) fn get_notice_exit_event(&self) -> u64 {
		self.noteevt.get_event()
	}


}

pub struct ThreadEvent {
	inner :Arc<RwLock<ThreadEventInner>>,
}

impl ThreadEvent {
	pub fn new() -> Result<Self,Box<dyn Error>> {
		let retv :Self = Self {
			inner : Arc::new(RwLock::new(ThreadEventInner::new()?)),
		};
		Ok(retv)
	}

	pub fn set_exit_event(&mut self) -> Result<(),Box<dyn Error>> {
		let mut cv = self.inner.write().unwrap();
		cv.set_exit_event()
	}

	pub fn set_notice_exit_event(&mut self) -> Result<(),Box<dyn Error>> {
		let mut cv = self.inner.write().unwrap();
		cv.set_notice_exit_event()		
	}

	pub fn get_exit_event(&self) -> u64 {
		let cv = self.inner.read().unwrap();
		cv.get_exit_event()
	}

	pub fn get_notice_exit_event(&self) -> u64 {
		let cv = self.inner.read().unwrap();
		cv.get_notice_exit_event()
	}

}

unsafe impl Send for ThreadEvent {}
unsafe impl Sync for ThreadEvent {}

pub (crate) struct EvtBody<T> {
	//b :Option<Arc<UnsafeCell<T>>>,
	b : Vec<T>,
	lock : RwLock<i32>,
}


pub struct EvtSyncUnsafeCell<T> {
    inner: UnsafeCell<T>,
}

unsafe impl<T> Sync for EvtSyncUnsafeCell<T> {}

impl<T> EvtSyncUnsafeCell<T> {
    /// Constructs a new instance of `EvtSyncUnsafeCell` which will wrap the specified value.
    #[inline]
    pub const fn new(value: T) -> Self {
        Self { inner: UnsafeCell::new(value), }
    }

}

impl<T> EvtSyncUnsafeCell<T> {
    /// Gets a mutable pointer to the wrapped value.
    ///
    /// This can be cast to a pointer of any kind.
    /// Ensure that the access is unique (no active references, mutable or not)
    /// when casting to `&mut T`, and ensure that there are no mutations
    /// or mutable aliases going on when casting to `&T`
    #[inline]
    pub const fn get(&self) -> *mut T {
        self.inner.get()
    }

}


impl<T> EvtBody<T> {
	pub (crate) fn new() -> Self {
		let retv :Self = Self {
			//b : Some(Arc::new(UnsafeCell::new(b))),
			b : Vec::new(),
			lock : RwLock::new(0),
		};
		retv
	}

	pub (crate) fn get(&mut self) -> Option<T> {
		let _cv = self.lock.read().unwrap();
		return self.b.pop();
	}

	pub (crate) fn push(&mut self,nb :T)  {
		let mut cv = self.lock.write().unwrap();
		*cv += 1;
		self.b.push(nb);
		return;
	}

}



unsafe impl<T> Send for EvtBody<T> {}
unsafe impl<T> Sync for EvtBody<T> {}

//pub struct EvtThreadInner<F,T> 
pub struct EvtThreadInner<T> {
	chld : Vec<JoinHandle<()>>,
	evts : ThreadEvent,
	started : bool,
	retval : EvtBody<T>,
}

impl<T> Drop for EvtThreadInner<T> {
	fn drop(&mut self) {
		self.close();
	}
}

impl<T> EvtThreadInner<T> {
	pub fn close(&mut self) {
		if self.started {
			let exitevt = self.evts.get_exit_event();
			let _ = self.evts.set_notice_exit_event();
			let mut cnt : u64 = 0;
			loop {
				let bval = wait_event_fd_timeout(exitevt,10);
				if bval {
					break;
				}
				let _ = self.evts.set_notice_exit_event();
				cnt += 1;
				if (cnt % 100) == 0 {
					evtcall_log_error!("wait thread cnt [{}]",cnt);
				}
			}
			if self.chld.len() > 0 {
				let o = self.chld.pop().unwrap();
				let _ = o.join();
			}
			self.started = false;
		}
	}
}

impl<T : 'static> EvtThreadInner<T> {
	pub fn new() -> Result<Self,Box<dyn Error>> {
		let retv : Self = Self {
			chld : Vec::new(),
			evts : ThreadEvent::new()?,
			started : false,
			retval : EvtBody::new(),
		};
		Ok(retv)
	}

	pub fn is_exited(&mut self) -> bool {
		if !self.started {
			return true;
		}
		let exitevt = self.evts.get_exit_event();
		let bval = wait_event_fd_timeout(exitevt,-1);
		return bval;
	}

	pub fn start<F :  FnOnce() -> T + 'static + Send + Sync>(&mut self,ncall :F, other :Arc<EvtSyncUnsafeCell<EvtThreadInner<T>>>) -> Result<(),Box<dyn Error>> {
		if !self.started {
			let cother = other.clone();
			let o = std::thread::spawn(move || {
				let retv = ncall();
				let refm :&mut EvtThreadInner<T> = unsafe {&mut *cother.get()}; 
				let _ = refm.evts.set_exit_event();				
				{					
					refm.retval.push(retv);	
				}
				()
			});
			self.chld.push(o);
			self.started = true;
		}
		Ok(())
	}

	pub fn get_return(&mut self) -> Option<T> {
		return self.retval.get();
	}

	pub fn stop(&mut self) -> Result<(),Box<dyn Error>> {
		if !self.started {
			return Ok(());
		}
		return self.evts.set_notice_exit_event();
	}
}