

use crate::consts::*;
use crate::interface::*;
use crate::timeop::*;
use std::sync::Arc;
use std::error::Error;
use std::collections::HashMap;
//use libc::{clock_gettime,CLOCK_MONOTONIC_COARSE,timespec,c_int};
use libc::{c_int};

use super::{evtcall_error_class,evtcall_new_error};

evtcall_error_class!{MainLoopLinuxError}


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

#[derive(Clone)]
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

pub struct EvtMain {
	evtmaps :HashMap<u64,EvtCallLinux>,
	timermaps :HashMap<u64,EvtTimerLinux>,
	guidevtmaps : HashMap<u64,u64>,
	guid : u64,
	epollfd : c_int,
	exited : i32,
}

impl Drop for EvtMain {
	fn drop(&mut self) {
		self.reset_all();
	}
}

impl EvtMain {
	pub fn new(_flags :u32) -> Result<Self,Box<dyn Error>> {
		let mut retv = Self {
			evtmaps : HashMap::new(),
			timermaps : HashMap::new(), 
			guidevtmaps : HashMap::new(),
			guid : 1,
			epollfd : -1,
			exited : 0,
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
		unsafe {
			evtid = (&(*(*b))).get_evt();
			let evttype = (&(*(*b))).get_evttype();
			let mut optype :u32 = 0;
			if (evttype & READ_EVENT) != 0 {
				optype |= libc::EPOLLIN as u32;
			}
			if (evttype & WRITE_EVENT) != 0 {
				optype |= libc::EPOLLOUT as u32;
			}
			if (evttype & ERROR_EVENT) != 0 {
				optype |= libc::EPOLLERR as u32;
			}
			if (evttype & ET_TRIGGER) != 0 {
				optype |= libc::EPOLLET as u32;
			}
			let mut evt :libc::epoll_event = libc::epoll_event {
				events : optype,
				u64 : evtid,
			};

			let retv = libc::epoll_ctl(self.epollfd,libc::EPOLL_CTL_ADD,evtid as i32,&mut evt);
			if retv < 0 {
				evtcall_new_error!{MainLoopLinuxError,"can not EPOLL_ADD error [{}]",retv}
			}
		}

		let ev = EvtCallLinux::new(bv)?;
		self.guid += 1;
		self.evtmaps.insert(self.guid,ev);
		self.guidevtmaps.insert(evtid, self.guid);
		Ok(())
	}

	#[allow(unused_variables)]
	pub fn remove_timer(&mut self,guid:u64) -> Result<(),Box<dyn Error>> {
		match self.timermaps.get(&guid) {
			Some(_v) => {

			},
			None => {
				evtcall_new_error!{MainLoopLinuxError,"not get timer {} timer",guid}
			}
		}
		self.timermaps.remove(&guid);
		Ok(())
	}

	#[allow(unused_variables)]
	pub fn remove_event(&mut self,bv :Arc<*mut dyn EvtCall>) -> Result<(),Box<dyn Error>> {
		let evtid :u64;
		let b = Arc::as_ptr(&bv);
		unsafe {
			evtid = (&(*(*b))).get_evt();
			let evttype = (&(*(*b))).get_evttype();
			let mut optype :u32 = 0;
			if (evttype & READ_EVENT) != 0 {
				optype |= libc::EPOLLIN as u32;
			}
			if (evttype & WRITE_EVENT) != 0 {
				optype |= libc::EPOLLOUT as u32;
			}
			if (evttype & ERROR_EVENT) != 0 {
				optype |= libc::EPOLLERR as u32;
			}
			if (evttype & ET_TRIGGER) != 0 {
				optype |= libc::EPOLLET as u32;
			}
			let mut evt :libc::epoll_event = libc::epoll_event {
				events : optype,
				u64 : evtid,
			};

			let retv = libc::epoll_ctl(self.epollfd,libc::EPOLL_CTL_DEL,evtid as i32,&mut evt);
			if retv < 0 {
				evtcall_new_error!{MainLoopLinuxError,"can not EPOLL_ADD error [{}]",retv}
			}
		}

		let curguid :u64;
		match self.guidevtmaps.get(&evtid) {
			Some(_v) => {
				curguid = *_v;
			},
			None => {
				evtcall_new_error!{MainLoopLinuxError,"cannot found 0x{:x} evtid",evtid}
			}
		}

		self.guidevtmaps.remove(&evtid);
		self.evtmaps.remove(&curguid);
		Ok(())
	}

	fn get_time(&self,maxtime :i32) -> i32 {
		let mut retv :i32 = maxtime;
		for (_, v) in self.timermaps.iter() {
			let cticks :u64 = get_cur_ticks();
			let curi = time_left(v.startticks,cticks,v.interval);
			if curi < 0 {
				return 1;
			}
			if curi < retv {
				retv = curi;
			}
		}
		return retv;
	}

	fn get_evts_guids(&self,evts :&[libc::epoll_event]) -> (Vec<u64>,Vec<u32>) {
		let mut retevguids :Vec<u64> = Vec::new();
		let mut retevtypes :Vec<u32> = Vec::new();
		let mut idx :usize = 0;
		let mut evtid :u64;
		while idx < evts.len() {
			evtid = evts[idx].u64 as u64;
			if evtid > 0 {
				match self.guidevtmaps.get(&evtid) {
					Some(v) => {
						match self.evtmaps.get(v) {
							Some(_ec) => {
								let mut evttype :u32 = 0;
								if (evts[idx].events & libc::EPOLLIN as u32) != 0 {
									evttype |= READ_EVENT;
								}
								if (evts[idx].events & libc::EPOLLOUT as u32) != 0 {
									evttype |= WRITE_EVENT;
								}
								if (evts[idx].events & libc::EPOLLERR as u32) != 0 {
									evttype |= ERROR_EVENT;
								}

								retevguids.push(*v);
								retevtypes.push(evttype);
							},
							None => {}
						}
					},
					None => {}
				}				
			}
			idx += 1;
		}
		return (retevguids,retevtypes);
	}

	fn get_timer_guids(&self) -> Vec<u64> {
		let mut rettvguids :Vec<u64> = Vec::new();
		for (guid, ev) in self.timermaps.iter() {
			let cticks = get_cur_ticks();
			let reti = time_left(ev.startticks,cticks,ev.interval);
			if reti < 0 {
				rettvguids.push(*guid);
			}
		}
		return rettvguids;
	}

	#[allow(unused_variables)]
	pub fn main_loop(&mut self) -> Result<(),Box<dyn Error>> {
		let mut evts :Vec<libc::epoll_event> = Vec::with_capacity(4);
		let mut retv :i32;
		let mut evtguids :Vec<u64>;
		let mut evttypes :Vec<u32>;
		let mut timerguids :Vec<u64>;		
		while self.exited == 0 {
			/*max 30 second*/
			let maxtime = self.get_time(30000);
			let reti :c_int;
			let mut idx :usize;
			let mut guid :u64;
			unsafe {
				reti = libc::epoll_wait(self.epollfd,  evts.as_ptr() as *mut libc::epoll_event,evts.capacity() as i32,maxtime as c_int);
			}

			evtguids = Vec::new();
			evttypes = Vec::new();
			if reti >= 0 {
				idx = reti as usize;
				while idx < evts.len() {
					evts[idx].u64 = 0;
					idx += 1;
				}

				(evtguids,evttypes) = self.get_evts_guids(&evts);
			}

			timerguids = self.get_timer_guids();

			idx = 0;
			while idx < evtguids.len() {
				guid = evtguids[idx];
				let mut findvk :Option<Arc<* mut dyn EvtCall>> = None;
				match self.evtmaps.get(&guid) {
					Some(ev) => {
						findvk = Some(ev.evt.clone());
					},
					None => {
					}
				}

				if findvk.is_some() {
					let c :Arc<* mut dyn EvtCall> = findvk.unwrap();
					let b = Arc::as_ptr(&c);
					let evttype :u32;

					evttype = evttypes[idx];
					if (evttype & READ_EVENT) != 0 {
						unsafe {
							(&mut (*(*b))).read(self)?;
						}
					}

					if (evttype & WRITE_EVENT) != 0 {
						unsafe {
							(&mut (*(*b))).write(self)?;
						}
					}

					if (evttype & ERROR_EVENT) != 0 {
						unsafe {
							(&mut (*(*b))).error(self)?;
						}
					}
				}

				idx += 1;
			}

			idx = 0;
			while idx < timerguids.len() {
				guid = timerguids[idx];
				let mut findtv :Option<EvtTimerLinux> = None;
				match self.timermaps.get(&guid) {
					Some(ev) => {
						findtv = Some(ev.clone());
					},
					None => {

					}
				}

				if findtv.is_some() {
					let c :EvtTimerLinux = findtv.unwrap();
					let b = Arc::as_ptr(&(c.timer));
					unsafe {
						(&mut (*(*b))).timer(self)?;
					}


					if !c.conti {
						self.timermaps.remove(&guid);
					} else {
						match self.timermaps.get_mut(&guid) {
							Some(cv) => {
								/*for next timer*/
								cv.startticks = get_cur_ticks();
							},
							None => {}
						}
					}
				}

				idx += 1;
			}

		}
		Ok(())
	}

	#[allow(unused_variables)]
	pub fn break_up(&mut self) -> Result<(),Box<dyn Error>> {
		self.exited = 1;
		Ok(())
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
		self.exited = 0;
	}
}

