

use crate::interface::*;
use std::sync::Arc;
use std::cell::UnsafeCell;
use std::error::Error;
use std::collections::HashMap;

#[allow(dead_code)]
pub (crate) struct EvtCallLinux {
	evts :Arc<UnsafeCell<dyn EvtCall>>,
	guid :u64,
	evttype: u32,
}

impl EvtCallLinux {
	fn new(bv :Arc<UnsafeCell<dyn EvtCall>>, eventtype :u32,guid :u64) -> Result<Self,Box<dyn Error>> {
		unsafe {
			let ptr = Arc::into_raw(bv.clone());
			Arc::increment_strong_count(ptr);
		}
		Ok(Self {
			evts : bv.clone(),
			guid : guid,
			evttype : eventtype,
		})
	}
}

impl Drop for EvtCallLinux {
	fn drop(&mut self) {
		unsafe {
			let ptr = Arc::into_raw(self.evts.clone());
			Arc::decrement_strong_count(ptr);
		}
	}
}

#[allow(dead_code)]
pub (crate) struct EvtTimerLinux {
	pub (crate) timer :Arc<UnsafeCell<dyn EvtTimer>>,
	pub (crate) startticks :u64,
	pub (crate) interval :i32,
	pub (crate) conti :bool,
	pub (crate) guid : u64,
}

impl EvtTimerLinux {
	fn new(v :Arc<UnsafeCell<dyn EvtTimer>>,interval :i32, conti :bool,guid :u64) -> Result<Self,Box<dyn Error>> {
		unsafe {
			/*we increment the strong count for it will give the safe used*/
			let ptr = Arc::into_raw(v.clone());
			Arc::increment_strong_count(ptr);
		}
		Ok(Self {
			timer : v.clone(),
			startticks : 0,
			interval : interval,
			conti : conti,
			guid : guid,
		})
	}
}

impl Drop for EvtTimerLinux {
	fn drop(&mut self) {
		unsafe {
			/*we have */
			let ptr = Arc::into_raw(self.timer.clone());
			Arc::decrement_strong_count(ptr);
		}		
	}
}

#[allow(dead_code)]
pub struct MainLoopLinux {
	evtsmap :HashMap<u64,EvtCallLinux>,
	evtfdmap :HashMap<u64,u64>,
	timersmap :HashMap<u64,EvtTimerLinux>,
	guid :u64,
}

impl MainLoopLinux {
	pub fn new() -> Result<Self,Box<dyn Error>> {
		Ok(Self {
			evtsmap  : HashMap::new(),
			evtfdmap : HashMap::new(),
			timersmap : HashMap::new(),
			guid : 1,
		})
	}

	pub fn add_timer(&mut self,bv :Arc<UnsafeCell<dyn EvtTimer>>,interval:i32,conti:bool) -> Result<u64,Box<dyn Error>> {
		self.guid += 1;
		let timer :EvtTimerLinux = EvtTimerLinux::new(bv,interval,conti,self.guid)?;
		self.timersmap.insert(self.guid,timer);
		Ok(self.guid)
	}

	#[allow(unused_variables)]
	pub fn add_event(&mut self,bv :Arc<UnsafeCell<dyn EvtCall>>, eventtype :u32) -> Result<(),Box<dyn Error>> {
		self.guid += 1;
		let ev :EvtCallLinux = EvtCallLinux::new(bv.clone(),eventtype,self.guid)?;
		self.evtsmap.insert(self.guid,ev);
		self.evtfdmap.insert(bv.get().get_evt(),self.guid);
		Ok(())
	}

	pub fn remove_timer(&mut self,guid:u64) -> Result<(),Box<dyn Error>> {
		self.timersmap.remove(&guid);
		Ok(())
	}

	pub fn remove_event(&mut self,_bv :Arc<UnsafeCell<dyn EvtCall>>) -> Result<(),Box<dyn Error>> {
		Ok(())
	}

	pub fn main_loop(&mut self) -> Result<(),Box<dyn Error>> {
		Ok(())
	}

	pub fn break_up(&mut self) -> Result<(),Box<dyn Error>> {
		Ok(())
	}
}

