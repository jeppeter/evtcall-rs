

use crate::interface::*;
use std::sync::Arc;
use std::cell::RefCell;
use std::error::Error;

#[allow(dead_code)]
pub (crate) struct EvtCallWindows {
	evts :Arc<RefCell<dyn EvtCall>>,
}

#[allow(dead_code)]
pub (crate) struct EvtTimerWindows {
	timer :Arc<RefCell<dyn EvtTimer>>,
	startticks :u64,
	interval :u32,
	conti :bool,
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