
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


impl EvtMain {
	#[cfg(windows)]
	pub fn new() -> Result<Self,Box<dyn Error>> {
		Ok(Self {
			ptr :MainLoopWindows::new()?,
		})
	}

	#[cfg(linux)]
	pub fn new() -> Result<Self,Box<dyn Error>> {
		Ok(Self {
			ptr :MainLoopLinux::new()?,
		})
	}


	pub fn add_timer(&mut self,_bv :Arc<RefCell<dyn EvtTimer>>) -> Result<(),Box<dyn Error>> {
		Ok(())
	}

	pub fn add_event(&mut self,_bv :Arc<RefCell<dyn EvtCall>>) -> Result<(),Box<dyn Error>> {
		Ok(())
	}

	pub fn remove_timer(&mut self,_bv :Arc<RefCell<dyn EvtTimer>>) -> Result<(),Box<dyn Error>> {
		Ok(())
	}

	pub fn remove_event(&mut self,_bv :Arc<RefCell<dyn EvtCall>>) -> Result<(),Box<dyn Error>> {
		Ok(())
	}

	pub fn main_loop(&mut self) -> Result<(),Box<dyn Error>> {
		Ok(())
	}

	pub fn break_up(&mut self) -> Result<(),Box<dyn Error>> {
		Ok(())
	}
}