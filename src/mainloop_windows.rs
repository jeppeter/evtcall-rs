

use crate::interface::*;
use std::sync::Arc;
use std::cell::UnsafeCell;
use std::error::Error;

#[allow(dead_code)]
pub (crate) struct EvtCallWindows {
	evts :Arc<UnsafeCell<dyn EvtCall>>,
}

#[allow(dead_code)]
pub (crate) struct EvtTimerWindows {
	timer :Arc<UnsafeCell<dyn EvtTimer>>,
	startticks :u64,
	interval :u32,
	conti :bool,
}

impl EvtTimerWindows {
	pub fn  new(bv :Arc<UnsafeCell<dyn EvtTimer>>, interval :i32 , conti :bool) -> Result<Self,Box<dyn Error>> {
		Ok(Self {
			timer :bv.clone(),
			startticks : 0,
			interval : interval as u32,
			conti : conti,
		})
	}
}

#[allow(dead_code)]
pub struct MainLoopWindows {
	evts :Vec<EvtCallWindows>,
	timers :Vec<EvtTimerWindows>,
}

impl MainLoopWindows {
	pub fn new() -> Result<Self,Box<dyn Error>> {
		Ok(Self {
			evts  : Vec::new(),
			timers : Vec::new(),
		})
	}

	pub fn add_timer(&mut self,bv :Arc<UnsafeCell<dyn EvtTimer>>,interval:i32,conti:bool) -> Result<(),Box<dyn Error>> {
		self.timers.push(EvtTimerWindows::new(bv,interval,conti)?);
		Ok(())
	}

	pub fn add_event(&mut self,_bv :Arc<UnsafeCell<dyn EvtCall>>) -> Result<(),Box<dyn Error>> {
		Ok(())
	}

	pub fn remove_timer(&mut self,_bv :Arc<UnsafeCell<dyn EvtTimer>>) -> Result<(),Box<dyn Error>> {
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