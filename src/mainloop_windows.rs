

use crate::interface::*;
use crate::consts::*;
use std::sync::Arc;
use std::cell::RefCell;
use std::error::Error;
use std::collections::HashMap;
use crate::timeop::*;

use winapi::um::winnt::{HANDLE,LPCWSTR};

use winapi::um::synchapi::{WaitForMultipleObjectsEx};
use winapi::shared::minwindef::{FALSE,DWORD,TRUE,BOOL};
use winapi::um::errhandlingapi::{GetLastError};
use winapi::um::minwinbase::{LPSECURITY_ATTRIBUTES,SECURITY_ATTRIBUTES};
use winapi::um::winbase::{WAIT_OBJECT_0};
use winapi::um::synchapi::{CreateEventW};
use winapi::um::handleapi::{CloseHandle};
use crate::{evtcall_error_class,evtcall_new_error,evtcall_log_error,evtcall_log_trace};
use crate::consts_windows::*;
use crate::logger::*;

evtcall_error_class!{MainLoopWindowsError}

macro_rules! get_errno {
	() => {{
		let mut retv :i32 ;
		unsafe {
			retv = GetLastError() as i32;
		}
		if retv != 0 {
			retv = -retv;
		} else {
			retv = -1;
		}
		retv
	}};
}


macro_rules! create_event_safe {
	($hd :expr,$name :expr) => {
		let _errval :i32;
		let _pattr :LPSECURITY_ATTRIBUTES = std::ptr::null_mut::<SECURITY_ATTRIBUTES>() as LPSECURITY_ATTRIBUTES;
		let _pstr :LPCWSTR = std::ptr::null() as LPCWSTR;
		$hd = unsafe {CreateEventW(_pattr,TRUE,FALSE,_pstr)};
		if $hd == NULL_HANDLE_VALUE {
			_errval = get_errno!();
			evtcall_new_error!{MainLoopWindowsError,"create {} error {}",$name,_errval}
		}
	};
}

macro_rules! close_handle_safe {
	($hdval : expr,$name :expr) => {
		let _bret :BOOL;
		let _errval :i32;
		if $hdval != NULL_HANDLE_VALUE {
			unsafe {
				_bret = CloseHandle($hdval);
			}
			if _bret == FALSE {
				_errval = get_errno!();
				evtcall_log_error!("CloseHandle {} error {}",$name,_errval);
			}
		}
		$hdval = NULL_HANDLE_VALUE;
	};
}


#[derive(Clone)]
struct EvtCallWindows {
	evt :Arc<RefCell<dyn EvtCall>>,
	evthd : u64,
	evttype : u32,
}

impl EvtCallWindows {
	fn new(av :Arc<RefCell<dyn EvtCall>>,evthd :u64, evttype :u32) -> Result<Self,Box<dyn Error>> {
		Ok(Self{
			evt : av.clone(),
			evthd : evthd,
			evttype : evttype,
		})
	}
}

#[derive(Clone)]
struct EvtTimerWindows {
	timer :Arc<RefCell<dyn EvtTimer>>,
	startticks :u64,
	interval : i32,
	conti :bool,
}

impl EvtTimerWindows {
	fn new(av :Arc<RefCell<dyn EvtTimer>>, interval : i32,conti :bool) -> Result<Self,Box<dyn Error>> {
		Ok(Self {
			timer : av.clone(),
			interval : interval,
			conti : conti,
			startticks : get_cur_ticks(),
		})
	}
}


pub struct EvtMain {
	timerevt :Vec<HANDLE>,
	evtmaps :HashMap<u64,EvtCallWindows>,
	timermaps :HashMap<u64,EvtTimerWindows>,
	guidevtmaps :HashMap<u64,u64>,
	guid : u64,
	exited : i32,
}

impl Drop for EvtMain {
	fn drop(&mut self) {
		self.close();
	}
}

impl EvtMain {
	pub fn new(_flags :u32) -> Result<Self,Box<dyn Error>> {
		let mut retv :Self = Self {
			timerevt : Vec::new(),
			evtmaps : HashMap::new(),
			timermaps : HashMap::new(),
			guidevtmaps : HashMap::new(),
			guid : 1,
			exited : 0,
		};
		retv.timerevt.push(NULL_HANDLE_VALUE);
		create_event_safe!(retv.timerevt[0],"timer event");
		Ok(retv)
	}

	pub fn add_timer(&mut self,bv :Arc<RefCell<dyn EvtTimer>>,interval:i32,conti:bool) -> Result<u64,Box<dyn Error>> {
		self.guid += 1;
		let ntimer :EvtTimerWindows = EvtTimerWindows::new(bv,interval,conti)?;
		evtcall_log_trace!("insert timer 0x{:x}",self.guid);
		self.timermaps.insert(self.guid,ntimer);
		Ok(self.guid)
	}

	pub fn add_event(&mut self,bv :Arc<RefCell<dyn EvtCall>>,evthd :u64,evttype :u32) -> Result<(),Box<dyn Error>> {

		if evthd == 0 || evthd == INVALID_EVENT_HANDLE {
			evtcall_new_error!{MainLoopWindowsError,"not valid evthd 0x{:x}",evthd}
		}

		self.guid += 1;
		evtcall_log_trace!("add evthd 0x{:x} evttype 0x{:x}",evthd,evttype);
		let nevt :EvtCallWindows = EvtCallWindows::new(bv,evthd,evttype)?;
		self.evtmaps.insert(self.guid,nevt);
		self.guidevtmaps.insert(evthd,self.guid);
		Ok(())
	}

	pub fn remove_timer(&mut self,guid:u64) -> i32 {
		let mut removed : i32 = 0;
		match self.timermaps.get(&guid) {
			Some(_ev) => {
			},
			None => {
				evtcall_log_error!("not get timer 0x{:x} timer",guid);
				return removed;
			}
		}
		evtcall_log_trace!("remove timer 0x{:x}",guid);
		self.timermaps.remove(&guid);
		removed = 1;
		return removed;
	}

	pub fn remove_event(&mut self,evthd :u64) -> i32 {
		let mut removed :i32 = 0;
		let curguid :u64;
		match self.guidevtmaps.get(&evthd) {
			Some(_v) => {
				curguid = *_v;
			},
			None => {
				evtcall_log_error!("cannot found 0x{:x} evtid",evthd);
				return removed;
			}
		}

		self.guidevtmaps.remove(&evthd);
		self.evtmaps.remove(&curguid);
		removed = 1;
		evtcall_log_trace!("remove evthd 0x{:x}",evthd);
		return removed;
	}

	fn get_handles(&self) -> (Vec<HANDLE>,Vec<u64>) {
		let mut rethdls :Vec<HANDLE> = Vec::new();
		let mut retguids :Vec<u64> = Vec::new();
		for (v,g) in self.guidevtmaps.iter() {
			rethdls.push(*v as HANDLE);
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

	fn _debug_mode_call(&mut self) {
		evtcall_log_trace!("will call debug_mode");
		for (_,v) in self.evtmaps.iter_mut() {
			let b = v.evt.clone();
			b.borrow_mut().debug_mode(file!(),line!());
		}
	}

	pub fn main_loop(&mut self) -> Result<(),Box<dyn Error>> {
		while self.exited == 0 {
			let (handles,guids)  = self.get_handles();
			let timeout = self.get_timeout(30000);
			let dret :DWORD;

			self._debug_mode_call();

			if handles.len() > 0 {
				/*for _h in handles.iter() {
					evtcall_log_trace!("h 0x{:x}",*_h as u64);
				}*/
				unsafe {
					dret = WaitForMultipleObjectsEx(handles.len() as DWORD,handles.as_ptr(),FALSE,timeout,FALSE);
				}				
			} else {
				evtcall_log_trace!("timer set");
				assert!(self.timerevt.len() > 0);
				unsafe {
					dret = WaitForMultipleObjectsEx(self.timerevt.len() as DWORD, self.timerevt.as_ptr(),FALSE,timeout,FALSE);
				}
			}

			let timeguids = self.get_time_guids();
			evtcall_log_trace!("dret 0x{:x}",dret);
			if dret >= WAIT_OBJECT_0 && dret < ( WAIT_OBJECT_0 + (handles.len() as DWORD)) {
				evtcall_log_trace!(" ");
				if guids.len() > (dret - WAIT_OBJECT_0) as usize {
					let curguid = guids[(dret as usize) - (WAIT_OBJECT_0 as usize)];
					//evtcall_log_trace!("hdl 0x{:x}",handles[(dret as usize) - (WAIT_OBJECT_0 as usize)] as u64);
					let mut findev :Option<EvtCallWindows> = None;
					match self.evtmaps.get(&curguid) {
						Some(ev) => {
							findev = Some(ev.clone());
						},
						None => {}
					}

					if findev.is_some() {
						let c = findev.unwrap();
						let b = c.evt.clone();
						let evttype :u32 = c.evttype;
						let hd :u64 = c.evthd;
						evtcall_log_trace!("b {:p}",b);
						b.borrow_mut().handle(hd,evttype,self)?;
					}					
				}
				//evtcall_log_trace!(" ");
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
					let b = c.timer.clone();
					b.borrow_mut().timer(*g,self)?;

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

	pub fn close(&mut self) {
		let mut guids :Vec<u64> = Vec::new();
		let mut idx :usize;

		for (k,_) in self.evtmaps.iter() {
			guids.push(*k);
		}

		idx = 0;
		while idx < guids.len() {
			let mut findev :Option<EvtCallWindows> = None;
			match self.evtmaps.get(&guids[idx]) {
				Some(ev) => {
					findev = Some(ev.clone());
				},
				None => {}
			}

			if findev.is_some() {
				let c = findev.unwrap();
				let b = c.evt.clone();
				let evttype :u32 = c.evttype;
				let hd :u64 = c.evthd;
				b.borrow_mut().close_event(hd,evttype,self);
			}
			idx += 1;
		}

		guids = Vec::new();
		for (k,_) in self.timermaps.iter() {
			guids.push(*k);
		}

		for g in guids.iter() {
			let  mut findtv :Option<EvtTimerWindows> = None;
			evtcall_log_trace!("will remove 0x{:x} timer",*g);
			match self.timermaps.get(g) {
				Some(cv) => {
					findtv = Some(cv.clone());
				},
				None => {},
			}

			if findtv.is_some() {
				let c = findtv.unwrap();
				let b = c.timer.clone();
				b.borrow_mut().close_timer(*g,self);
			}
		}


		for e in self.timerevt.iter_mut() {
			close_handle_safe!(*e,"timerevt");
		}
		self.timerevt = Vec::new();
		self.guid = 1;
		self.exited = 0;
		self.evtmaps = HashMap::new();
		self.timermaps = HashMap::new();
		self.guidevtmaps = HashMap::new();		
	}
}