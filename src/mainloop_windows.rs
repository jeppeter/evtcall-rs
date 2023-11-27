

use crate::interface::*;
use std::sync::Arc;
use std::error::Error;
use std::collections::HashMap;
use crate::timeop::*;
use crate::consts::*;

use winapi::um::winnt::{HANDLE};
use winapi::um::synchapi::{WaitForMultipleObjectsEx};
use winapi::shared::miniwindef::{FALSE,DWORD};

use super::{evtcall_error_class,evtcall_new_error};

evtcall_error_class!{MainLoopWindowsError}


#[derive(Clone)]
struct EvtCallWindows {
	evt :Arc<*mut dyn EvtCall>,
}

impl EvtCallWindows {
	fn new(av :Arc<* mut dyn EvtCall>) -> Result<Self,Box<dyn Error>> {
		Ok(Self{
			evt : av,
		})
	}
}

#[derive(Clone)]
struct EvtTimerWindows {
	timer :Arc<*mut dyn EvtTimer>,
	startticks :u64,
	interval : i32,
	conti :bool,
}

impl EvtTimerWindows {
	fn new(av :Arc<* mut dyn EvtTimer>, interval : i32,conti :bool) -> Result<Self,Box<dyn Error>> {
		Ok(Self {
			timer : av,
			interval : interval,
			conti : conti,
			startticks : get_cur_ticks(),
		})
	}
}


pub struct EvtMain {
	evtmaps :HashMap<u64,EvtCallWindows>,
	timermaps :HashMap<u64,EvtTimerWindows>,
	guidevtmaps :HashMap<u64,u64>,
	guid : u64,
	exited : i32,
}

impl Drop for EvtMain {
	fn drop(&mut self) {
		self.reset_all();
	}
}

impl EvtMain {
	pub fn new() -> Result<Self,Box<dyn Error>> {
		Ok(Self {
			evtmaps : HashMap::new(),
			timermaps : HashMap::new(),
			guidevtmaps : HashMap::new(),
			guid : 1,
			exited : 0,
		})
	}

	pub fn add_timer(&mut self,bv :Arc<*mut dyn EvtTimer>,interval:i32,conti:bool) -> Result<u64,Box<dyn Error>> {
		self.guid += 1;
		let ntimer :EvtTimerWindows = EvtTimerWindows::new(bv,interval,conti)?;
		self.timermaps.insert(self.guid,ntimer);
		Ok(self.guid)
	}

	#[allow(unused_variables)]
	pub fn add_event(&mut self,bv :Arc<*mut dyn EvtCall>) -> Result<(),Box<dyn Error>> {
		self.guid += 1;
		let b = Arc::as_ptr(&bv);
		let evtid :u64;
		unsafe {
			evtid = (&(*(*b))).get_evt();
		}
		let nevt :EvtCallWindows = EvtCallWindows::new(bv)?;
		self.evtmaps.insert(self.guid,nevt);
		self.guidevtmaps.insert(evtid,self.guid);
		Ok(())
	}

	pub fn remove_timer(&mut self,guid:u64) -> Result<(),Box<dyn Error>> {
		match self.timermaps.get(&guid) {
			Some(_ev) => {
			},
			None => {
				evtcall_new_error!{MainLoopWindowsError,"not get timer {} timer",guid}
			}
		}
		self.timermaps.remove(&guid);
		Ok(())
	}

	pub fn remove_event(&mut self,bv :Arc<*mut dyn EvtCall>) -> Result<(),Box<dyn Error>> {
		let evtid :u64;
		let b = Arc::as_ptr(&bv);
		unsafe {
			evtid = (&(*(*b))).get_evt();
		}

		let curguid :u64;
		match self.guidevtmaps.get(&evtid) {
			Some(_v) => {
				curguid = *_v;
			},
			None => {
				evtcall_new_error!{MainLoopWindowsError,"cannot found 0x{:x} evtid",evtid}
			}
		}

		self.guidevtmaps.remove(&evtid);
		self.evtmaps.remove(&curguid);
		Ok(())
	}

	fn get_handles(&self) -> (Vec<HANDLE>,Vec<u64>) {
		let mut rethdls :Vec<HANDLE> = Vec::new();
		let mut retguids :Vec<u64> = Vec::new();
		for (g,v) in self.evtmaps.iter() {
			let b = Arc::as_ptr(&v.evt);
			let evtid :u64;
			unsafe {
				evtid = (&(*(*b))).get_evt();
			}

			rethdls.push(evtid as HANDLE);
			retguids.push(*g);
		}

		return (rethdls,retguids);
	}

	fn get_timeout(&self, maxtime :u32) -> u32 {
		let mut retv :u32 = maxtime;
		for (_, v) in self.timermaps.iter() {
			let cticks = get_cur_ticks();
			let reti = time_left(v.startticks,cticks,v.interval);
			if reti < 0 {
				return 1;
			}
			if (reti as u32) < retv {
				retv = reti as u32;
			}
		}
		return retv;
	}

	fn get_time_guids(&self) -> Vec<u64> {
		let mut retv :Vec<u64> = Vec::new();
		for (g,v) in self.timermaps.iter() {
			let cticks = get_cur_ticks();
			let reti = time_left(v.startticks,cticks,v.interval);
			if reti < 0 {
				retv.push(*g);
			}
		}
		return retv;
	}

	pub fn main_loop(&mut self) -> Result<(),Box<dyn Error>> {
		while self.exited == 0 {
			let (handles,guids)  = self.get_handles();
			let timeout = self.get_timeout(30000);
			let dret :DWORD;

			unsafe {
				dret = WaitForMultipleObjectsEx(handles.len(),handles.as_ptr(),FALSE,timeout,FALSE);
			}

			let timeguids = self.get_time_guids();

			if dret >= 0 && dret < handles.len() {
				let curguid = guids[dret];
				let mut findev :Option<EvtCallWindows> = None;
				match self.evtmaps.get(&curguid) {
					Some(ev) => {
						findev = Some(ev.clone());
					},
					None => {}
				}

				if findev.is_some() {
					let c = findev.unwrap();
					let b = Arc::as_ptr(&c.evt);
					unsafe {
						let evttype = (&(*(*b))).get_evttype();
						if (evttype & READ_EVENT) != 0 {
							(&mut (*(*b))).read(self)?;
						}

						if (evttype & WRITE_EVENT) != 0 {
							(&mut (*(*b))).write(self)?;
						}
						if (evttype & ERROR_EVENT) != 0 {
							(&mut (*(*b))).error(self)?;
						}
					}					
				}
			}

			for g in timeguids.iter() {
				let  mut findtv :Option<EvtTimerWindows> = None;
				match self.timermaps.get(g) {
					Some(cv) => {
						findtv = Some(cv.clone());
					},
					None => {}
				}

				if findtv.is_some() {
					let c = findtv.unwrap();
					let b = Arc::as_ptr(&c.timer);
					unsafe {
						(&mut (*(*b))).timer(self)?;	
					}

					if c.conti {
						match self.timermaps.get_mut(g) {
							Some(cv) => {
								cv.startticks = get_cur_ticks();
							},
							None => {}
						}
					} else {
						self.timermaps.remove(g);
					}
				}
			}

		}
		Ok(())
	}

	pub fn break_up(&mut self) -> Result<(),Box<dyn Error>> {
		self.exited = 1;
		Ok(())
	}

	pub fn reset_all(&mut self) {
		self.guid = 1;
		self.exited = 0;
		self.evtmaps = HashMap::new();
		self.timermaps = HashMap::new();
		self.guidevtmaps = HashMap::new();		
	}
}