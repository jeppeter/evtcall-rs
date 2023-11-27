

use crate::interface::*;
use std::sync::Arc;
use std::error::Error;
#[allow(unused_imports)]
use std::collections::HashMap;
use libc::{clock_gettime,CLOCK_MONOTONIC_COARSE,timespec,c_int};

use super::{evtcall_error_class,evtcall_new_error};

evtcall_error_class!{MainLoopLinuxError}

fn get_cur_ticks() -> u64 {
	let mut  curtime = timespec {
		tv_sec : 0,
		tv_nsec : 0,
	};
	unsafe {clock_gettime(CLOCK_MONOTONIC_COARSE,&mut curtime);};
	let mut retmills : u64 = 0;
	retmills += (curtime.tv_sec as u64 )  * 1000;
	retmills += ((curtime.tv_nsec as u64) % 1000000000) / 1000000;
	return retmills;
}

struct EvtCallLinux {
	evt :Arc<*mut dyn EvtCall>,
}

impl EvtCallLinux {
	fn new(av :Arc<* mut dyn EvtCall>) -> Result<Self,Box<dyn Error>> {
		Ok(Self{
			evt : av,
		})
	}
}

struct EvtTimerLinux {
	timer :Arc<*mut dyn EvtTimer>,
	startticks :u64,
	interval : i32,
	conti :bool,
}

impl EvtTimerLinux {
	fn new(av :Arc<* mut dyn EvtTimer>, interval : i32,conti :bool) -> Result<Self,Box<dyn Error>> {
		Ok(Self {
			timer : av,
			interval : interval,
			conti : conti,
			startticks : get_cur_ticks(),
		})
	}
}

#[allow(dead_code)]
pub struct MainLoopLinux {
	evtmaps :HashMap<u64,EvtCallLinux>,
	timermaps :HashMap<u64,EvtTimerLinux>,
	guidevtmaps : HashMap<u64,u64>,
	guid : u64,
	epollfd : c_int,
}

impl Drop for MainLoopLinux {
	fn drop(&mut self) {
		self.reset_all();
	}
}

impl MainLoopLinux {
	pub fn new(_flags :u32) -> Result<Self,Box<dyn Error>> {
		let mut retv = Self {
			evtmaps : HashMap::new(),
			timermaps : HashMap::new(), 
			guidevtmaps : HashMap::new(),
			guid : 1,
			epollfd : -1,
		};
		retv.epollfd = unsafe {libc::epoll_create1(0)};
		if retv.epollfd < 0 {
			evtcall_new_error!{MainLoopLinuxError,"cannot epoll_create1"}
		}

		Ok(retv)
	}

	#[allow(unused_variables)]
	pub fn add_timer(&mut self,bv :Arc<*mut dyn EvtTimer>,interval:i32,conti:bool) -> Result<u64,Box<dyn Error>> {
		self.guid += 1;
		let ntimer :EvtTimerLinux = EvtTimerLinux::new(bv,interval,conti)?;
		self.timermaps.insert(self.guid,ntimer);
		Ok(self.guid)
	}

	#[allow(unused_variables)]
	pub fn add_event(&mut self,bv :Arc<*mut dyn EvtCall>) -> Result<(),Box<dyn Error>> {

		let evtid :u64 ;
		let b = Arc::as_ptr(&bv);
		unsafe { evtid = (&(*(*b))).get_evt();}
		let ev = EvtCallLinux::new(bv)?;
		self.guid += 1;
		self.evtmaps.insert(self.guid,ev);
		self.guidevtmaps.insert(evtid, self.guid);
		Ok(())
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
		if self.epollfd >= 0 {
			unsafe {
				libc::close(self.epollfd);	
			}			
		}
		self.epollfd = -1;
		self.evtmaps = HashMap::new();
		self.guidevtmaps = HashMap::new();
		self.timermaps = HashMap::new();
		self.guid = 1;
	}
}

