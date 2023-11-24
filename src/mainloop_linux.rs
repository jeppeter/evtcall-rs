

use crate::interface::*;
use std::sync::Arc;
use std::error::Error;
#[allow(unused_imports)]
use std::collections::HashMap;
extern crate libc;

fn get_cur_ticks() -> u64 {
	let mut  curtime = libc::timespec {
		tv_sec : 0,
		tv_nsec : 0,
	};
	let ret = unsafe {libc::clock_gettime(libc::CLOCK_MONOTONIC_COARSE,&mut curtime);};
	let mut retmills : u64 = 0;
	retmills += (curtime.tv_sec as u64 )  * 1000;
	retmills += ((curtime.tv_nsec as u64) % 1000000000) / 1000000;
	return retmills;
}

struct EvtCallLinux {
	evt :Arc<*mut dyn EvtCall>,
}

struct EvtTimerLinux {
	timer :Arc<*mut dyn EvtTimer>,
	startticks :u64,
	interval : i32,
	conti :bool,
}

#[allow(dead_code)]
pub struct MainLoopLinux {
	evtmaps :HashMap<u64,EvtCallLinux>,
}

impl MainLoopLinux {
	pub fn new() -> Result<Self,Box<dyn Error>> {
		Ok(Self {
			evtmaps : HashMap::new(),
		})
	}

	#[allow(unused_variables)]
	pub fn add_timer(&mut self,bv :Arc<*mut dyn EvtTimer>,interval:i32,conti:bool) -> Result<u64,Box<dyn Error>> {
		unimplemented!()
	}

	#[allow(unused_variables)]
	pub fn add_event(&mut self,bv :Arc<*mut dyn EvtCall>) -> Result<(),Box<dyn Error>> {
		unimplemented!()
	}

	#[allow(unused_variables)]
	pub fn remove_timer(&mut self,guid:u64) -> Result<(),Box<dyn Error>> {
		unimplemented!()
	}

	#[allow(unused_variables)]
	pub fn remove_event(&mut self,bv :Arc<*mut dyn EvtCall>) -> Result<(),Box<dyn Error>> {
		unimplemented!()
	}

	#[allow(unused_variables)]
	pub fn main_loop(&mut self) -> Result<(),Box<dyn Error>> {
		unimplemented!()
	}

	#[allow(unused_variables)]
	pub fn break_up(&mut self) -> Result<(),Box<dyn Error>> {
		unimplemented!()
	}

	#[allow(unused_variables)]
	pub fn reset_all(&mut self) {
		unimplemented!()
	}
}

