
//use crate::consts::*;
use std::thread::{JoinHandle};
use crate::*;
#[allow(unused_imports)]
use crate::logger::*;
use std::error::Error;
use std::cell::UnsafeCell;
use std::sync::{Arc,RwLock};
use crate::eventfd::*;
use crate::consts::*;
evtcall_error_class!{EvtThreadError}


struct ThreadEventInner {
	chldevt : EventFd,
	parentevt : EventFd,
}

impl ThreadEventInner {
	pub (crate) fn new() -> Result<Self,Box<dyn Error>> {
		let retv :Self = Self {
			chldevt : EventFd::new(0,EVENT_NO_AUTO_RESET,"child event")?,
			parentevt : EventFd::new(0,EVENT_NO_AUTO_RESET,"parent event")?,
		};
		retv.chldevt.debug_self(file!(),line!());
		retv.parentevt.debug_self(file!(),line!());
		Ok(retv)
	}

	pub (crate) fn get_child_evtfd(&self) -> EventFd {
		let cv = self.chldevt.clone();
		cv.debug_self(file!(),line!());
		cv
	}

	pub (crate) fn get_parent_evtfd(&self) -> EventFd {
		let cv = self.parentevt.clone();
		cv.debug_self(file!(),line!());
		cv
	}

}

#[derive(Clone)]
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

	pub fn get_child_evtfd(&self) -> EventFd {
		let cv = self.inner.read().unwrap();
		cv.get_child_evtfd()
	}

	pub fn get_parent_evtfd(&self) -> EventFd {
		let cv = self.inner.read().unwrap();
		cv.get_parent_evtfd()
	}
}

unsafe impl Send for ThreadEvent {}
unsafe impl Sync for ThreadEvent {}

pub (crate) struct EvtBody<T> {
	//b :Option<Arc<UnsafeCell<T>>>,
	b : Vec<T>,
	lock : RwLock<i32>,
}


pub (crate) struct EvtSyncUnsafeCell<T> {
	inner: UnsafeCell<T>,
}

unsafe impl<T> Sync for EvtSyncUnsafeCell<T> {}

impl<T> EvtSyncUnsafeCell<T> {
    /// Constructs a new instance of `EvtSyncUnsafeCell` which will wrap the specified value.
    #[inline]
    pub (crate)  const fn new(value: T) -> Self {
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
    pub (crate) const fn get(&self) -> *mut T {
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
		evtcall_log_trace!("call EvtThreadInner close");
		if self.started {
			let chldevt : EventFd = self.evts.get_child_evtfd();
			let parentevt :EventFd = self.evts.get_parent_evtfd();
			let _ = parentevt.set_event();
			let mut cnt : u64 = 0;
			loop {
				let bval = wait_event_fd_timeout(chldevt.get_event(),10);
				if bval {
					break;
				}
				let _ = parentevt.set_event();
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
	pub (crate) fn new(threvt :ThreadEvent) -> Result<Self,Box<dyn Error>> {
		let retv : Self = Self {
			chld : Vec::new(),
			evts : threvt,
			started : false,
			retval : EvtBody::new(),
		};
		Ok(retv)
	}

	pub (crate) fn is_exited(&mut self) -> bool {
		if !self.started {
			return true;
		}
		let chldevt = self.evts.get_child_evtfd();
		let bval = wait_event_fd_timeout(chldevt.get_event(),-1);
		return bval;
	}

	pub (crate) fn start<F :  FnOnce() -> T + 'static + Send + Sync>(&mut self,ncall :F, other :Arc<EvtSyncUnsafeCell<EvtThreadInner<T>>>) -> Result<(),Box<dyn Error>> {
		if !self.started {
			let cother = other.clone();
			evtcall_log_trace!("before spawn");
			let o = std::thread::spawn(move || {
				evtcall_log_trace!("before child call");
				let retv = ncall();
				let refm :&mut EvtThreadInner<T> = unsafe {&mut *cother.get()}; 
				let chldevt :EventFd = refm.evts.get_child_evtfd();
				let _ = chldevt.set_event();				
				{					
					refm.retval.push(retv);	
				}
				evtcall_log_trace!("after child call");
				()
			});
			self.chld.push(o);
			self.started = true;
			evtcall_log_trace!("after spawn");
		}
		Ok(())
	}

	pub (crate) fn get_return(&mut self) -> Option<T> {
		return self.retval.get();
	}

	pub (crate) fn stop(&mut self) -> Result<(),Box<dyn Error>> {
		if !self.started {
			return Ok(());
		}
		return self.evts.get_parent_evtfd().set_event();
	}

	pub (crate) fn try_join(&mut self, mills :i32) -> bool {
		if !self.started {
			return true;
		}
		let mut curval :i32 = mills;
		let chldevt :EventFd = self.evts.get_child_evtfd();
		let parentevt :EventFd = self.evts.get_parent_evtfd();
		
		if curval < 0 {
			/*for on second*/
			curval = 1000;
		}
		loop {
			let _ = parentevt.set_event();
			let bval = wait_event_fd_timeout(chldevt.get_event(),curval);
			if bval {
				break;
			}
			if mills >= 0 {
				/*we try again not*/
				return false;
			}
			/*now we should wait all*/			
		}
		if self.chld.len() > 0 {
			let o = self.chld.pop().unwrap();
			let _ = o.join();
		}
		self.started = false;
		return true;
	}
}

#[derive(Clone)]
pub struct EvtThread<T> {
	inner : Arc<EvtSyncUnsafeCell<EvtThreadInner<T>>>,
}

impl<T> Drop for EvtThread<T> {
	fn drop(&mut self) {
		self.close();
	}
}

impl<T> EvtThread<T> {
	pub fn close(&mut self) {
		evtcall_log_trace!("EvtThread close cnt {}",Arc::strong_count(&self.inner));
	}
}

impl<T : 'static>  EvtThread<T> {
	pub fn new(threvt :ThreadEvent) -> Result<Self,Box<dyn Error>> {
		let retv : Self = Self {
			inner : Arc::new(EvtSyncUnsafeCell::new(EvtThreadInner::new(threvt)?)),
		};
		Ok(retv)
	}

	pub fn is_exited(&mut self) -> bool {
		let refm :&mut EvtThreadInner<T> = unsafe {&mut *self.inner.get()}; 
		refm.is_exited()
	}

	pub fn start<F :  FnOnce() -> T + 'static + Send + Sync>(&mut self,ncall :F) -> Result<(),Box<dyn Error>> {
		let o = self.inner.clone();
		let refm :&mut EvtThreadInner<T> = unsafe {&mut *self.inner.get()}; 
		refm.start(ncall,o)
	}

	pub fn get_return(&mut self) -> Option<T> {
		let refm :&mut EvtThreadInner<T> = unsafe {&mut *self.inner.get()}; 
		return refm.get_return();
	}

	pub fn stop(&mut self) -> Result<(),Box<dyn Error>> {
		let refm :&mut EvtThreadInner<T> = unsafe {&mut *self.inner.get()}; 
		return refm.stop();
	}

	pub fn try_join(&mut self,mills :i32) -> bool {
		let refm :&mut EvtThreadInner<T> = unsafe {&mut *self.inner.get()}; 
		return refm.try_join(mills);
	}
}