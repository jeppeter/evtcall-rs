
use crate::interface::*;
use std::error::Error;
use std::cell::UnsafeCell;
use std::sync::Arc;

#[cfg(target_os = "windows")]
use crate::mainloop_windows::*;

#[cfg(target_os = "linux")]
use crate::mainloop_linux::*;

#[cfg(target_os = "windows")]
pub struct EvtMain {
	ptr :MainLoopWindows,
}


#[cfg(target_os = "linux")]
pub struct EvtMain {
	ptr :MainLoopLinux,
}


#[cfg(target_os = "windows")]
impl EvtMain {
	pub fn new() -> Result<Self,Box<dyn Error>> {
		Ok(Self {
			ptr :MainLoopWindows::new()?,
		})
	}
}


impl EvtMain {
	pub fn add_timer(&mut self,bv :Arc<UnsafeCell<dyn EvtTimer>>,interval:i32,conti:bool) -> Result<u64,Box<dyn Error>> {
		return self.ptr.add_timer(bv,interval,conti);
	}

	pub fn add_event(&mut self,bv :Arc<UnsafeCell<dyn EvtCall>>, eventtype :u32) -> Result<(),Box<dyn Error>> {
		return self.ptr.add_event(bv,eventtype);
	}

	pub fn remove_timer(&mut self,guid:u64) -> Result<(),Box<dyn Error>> {
		return self.ptr.remove_timer(guid);
	}

	pub fn remove_event(&mut self,bv :Arc<UnsafeCell<dyn EvtCall>>) -> Result<(),Box<dyn Error>> {
		return self.ptr.remove_event(bv);
	}

	pub fn main_loop(&mut self) -> Result<(),Box<dyn Error>> {
		return self.ptr.main_loop();
	}

	pub fn break_up(&mut self) -> Result<(),Box<dyn Error>> {
		return self.ptr.break_up();
	}
}

#[cfg(target_os = "linux")]
impl EvtMain {
	pub fn new() -> Result<Self,Box<dyn Error>> {
		Ok(Self {
			ptr :MainLoopLinux::new()?,
		})
	}
}

