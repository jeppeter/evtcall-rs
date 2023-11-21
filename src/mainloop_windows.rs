

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
}