
//use crate::consts::*;
use std::thread::{JoinHandle};
use crate::*;
use std::error::Error;
use std::cell::UnsafeCell;
use std::sync::{Arc,RwLock};
evtcall_error_class!{EvtThreadError}


#[cfg(target_os = "windows")]
include!("thread_windows.rs");

#[cfg(target_os = "linux")]
include!("thread_linux.rs");

unsafe impl Send for ThreadEvent {}
unsafe impl Sync for ThreadEvent {}

pub (crate) struct EvtBody<T> {
	//b :Option<Arc<UnsafeCell<T>>>,
	b : Option<Vec<T>>,
	hasvalue : bool,
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
	pub (crate) fn new(nb :T) -> Self {
		let nv = vec![nb];
		let retv :Self = Self {
			//b : Some(Arc::new(UnsafeCell::new(b))),
			b : Some(nv),
			hasvalue : true,
		};
		retv
	}

	pub (crate) fn get(&mut self) -> T {
		if !self.hasvalue {
			panic!("cannot be here");
		}
		let retb = self.b.as_mut().unwrap().pop();
		self.b = None;
		self.hasvalue = false;
		return retb.unwrap();
	}


	pub (crate) fn is_ready(&self) -> bool {
		return self.hasvalue;
	}
}



unsafe impl<T> Send for EvtBody<T> {}
unsafe impl<T> Sync for EvtBody<T> {}

pub struct EvtThreadInner<F,T> 
	where 
	//T : std::marker::Send + std::marker::Sync,
	T: Send + 'static,
	F : Fn() -> T,
	F : Send + 'static,
	F : Sync + 'static, {
	chld : Option<JoinHandle<()>>,
	callfn : Arc<F>,
	evts : ThreadEvent,
	started : bool,
	retval : Option<EvtBody<T>>,
	retlock : Arc<RwLock<i32>>,
}


impl<F,T> EvtThreadInner<F,T>
	where 
	//T : std::marker::Send + std::marker::Sync,
	T: Send + 'static,
	//F : FnOnce() -> T,
	F : Fn() -> T,
	F : Send + 'static,
	F : Sync + 'static,	 {
	pub fn new(callfn :F) -> Result<Self,Box<dyn Error>> {
		let retv : Self = Self {
			chld : None,
			callfn : Arc::new(callfn),
			evts : ThreadEvent::new()?,
			started : false,
			retval : None,
			retlock : Arc::new(RwLock::new(0)),
		};
		Ok(retv)
	}

	pub fn start(&mut self, other :Arc<EvtSyncUnsafeCell<EvtThreadInner<F,T>>>) -> Result<(),Box<dyn Error>> {
		if !self.started {
			let cother = other.clone();
			let o = std::thread::spawn(move || {
				let refc :&EvtThreadInner<F,T> =unsafe {&(*cother.get())};
				let ncall = refc.callfn.clone();
				let retv = ncall();
				let refm :&mut EvtThreadInner<F,T> = unsafe {&mut *cother.get()}; 
				let _ = refm.evts.set_exited();
				{
					let mut cv = refm.retlock.write().unwrap();
					refm.retval = Some(EvtBody::new(retv));	
					*cv += 1;
				}
				
				()
			});
			self.chld = Some(o);
			self.started = true;
		}
		Ok(())
	}

	pub fn get_return(&mut self) -> Result<T,Box<dyn Error>> {
		let mut cv = self.retlock.write().unwrap();
		if self.retval.is_none() ||  !self.retval.as_ref().unwrap().is_ready() {
			evtcall_new_error!{EvtThreadError,"not ready for return"}
		}
		*cv += 1;

		Ok(self.retval.as_mut().unwrap().get())
	}
}