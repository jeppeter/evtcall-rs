
use crate::interface::*;
use std::error::Error;
use std::cell::RefCell;
use std::sync::Arc;

#[cfg(windows)]
use crate::mainloop_windows::*;

#[cfg(linux)]
use crate::mainloop_linux::*;

#[allow(dead_code)]
#[cfg(windows)]
pub struct EvtMain {
	ptr :MainLoopWindows,
}


#[allow(dead_code)]
#[cfg(linux)]
pub struct EvtMain {
	ptr :MainLoopLinux,
}


#[cfg(windows)]
impl EvtMain {
	#[cfg(windows)]
	pub fn new() -> Result<Self,Box<dyn Error>> {
		Ok(Self {
			ptr :MainLoopWindows::new()?,
		})
	}

	pub fn add_timer(&mut self,bv :Arc<RefCell<dyn EvtTimer>>) -> Result<(),Box<dyn Error>> {
		return self.ptr.add_timer(bv);
	}

	pub fn add_event(&mut self,bv :Arc<RefCell<dyn EvtCall>>) -> Result<(),Box<dyn Error>> {
		return self.ptr.add_event(bv);
	}

	pub fn remove_timer(&mut self,bv :Arc<RefCell<dyn EvtTimer>>) -> Result<(),Box<dyn Error>> {
		return self.ptr.remove_timer(bv);
	}

	pub fn remove_event(&mut self,bv :Arc<RefCell<dyn EvtCall>>) -> Result<(),Box<dyn Error>> {
		return self.ptr.remove_event(bv);
	}

	pub fn main_loop(&mut self) -> Result<(),Box<dyn Error>> {
		return self.ptr.main_loop();
	}

	pub fn break_up(&mut self) -> Result<(),Box<dyn Error>> {
		return self.ptr.break_up();
	}
}

#[cfg(linux)]
impl EvtMain {
	pub fn new() -> Result<Self,Box<dyn Error>> {
		Ok(Self {
			ptr :MainLoopLinux::new()?,
		})
	}

	pub fn add_timer(&mut self,bv :Arc<RefCell<dyn EvtTimer>>) -> Result<(),Box<dyn Error>> {
		return self.ptr.add_timer(bv);
	}

	pub fn add_event(&mut self,bv :Arc<RefCell<dyn EvtCall>>) -> Result<(),Box<dyn Error>> {
		return self.ptr.add_event(bv);
	}

	pub fn remove_timer(&mut self,bv :Arc<RefCell<dyn EvtTimer>>) -> Result<(),Box<dyn Error>> {
		return self.ptr.remove_timer(bv);
	}

	pub fn remove_event(&mut self,bv :Arc<RefCell<dyn EvtCall>>) -> Result<(),Box<dyn Error>> {
		return self.ptr.remove_event(bv);
	}

	pub fn main_loop(&mut self) -> Result<(),Box<dyn Error>> {
		return self.ptr.main_loop();
	}

	pub fn break_up(&mut self) -> Result<(),Box<dyn Error>> {
		return self.ptr.break_up();
	}
}