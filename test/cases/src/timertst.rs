#[allow(unused_imports)]
use extargsparse_codegen::{extargs_load_commandline,ArgSet,extargs_map_function};
#[allow(unused_imports)]
use extargsparse_worker::{extargs_error_class,extargs_new_error};
#[allow(unused_imports)]
use extargsparse_worker::namespace::{NameSpaceEx};
#[allow(unused_imports)]
use extargsparse_worker::argset::{ArgSetImpl};
use extargsparse_worker::parser::{ExtArgsParser};
use extargsparse_worker::funccall::{ExtArgsParseFunc};


use std::cell::{RefCell,UnsafeCell};
use std::sync::Arc;
use std::error::Error;
use std::boxed::Box;
#[allow(unused_imports)]
use regex::Regex;
#[allow(unused_imports)]
use std::any::Any;

use lazy_static::lazy_static;
use std::collections::HashMap;

#[allow(unused_imports)]
use extlog::{debug_trace,debug_buffer_trace,format_buffer_log,format_str_log,debug_error};
#[allow(unused_imports)]
use extlog::loglib::{log_get_timestamp,log_output_function};
use super::strop::{parse_u64};

use evtcall::interface::*;
use evtcall::consts::*;
use evtcall::mainloop::EvtMain;
//use evtcall::sockhdl::{TcpSockHandle,init_socket,fini_socket};
//use std::io::{Write};
use super::logtrans::{init_log};

use super::exithdl::*;
use super::exithdl_consts::*;

extargs_error_class!{TimerTestError}

struct TimerTestInner {
	timerguid :u64,
	mills : i32,
	cnt : i32,
	times : i32,
	exithd : u64,
	insertexit : bool,
	evtmain : *mut EvtMain,
}

#[derive(Clone)]
struct TimerTest {
	inner : Arc<UnsafeCell<TimerTestInner>>,
}

impl Drop for TimerTestInner {
	fn drop(&mut self) {
		self.close();
	}
}

impl TimerTestInner {
	fn close(&mut self) {
		self.close_inner();
	}

	fn close_inner(&mut self) {
		if self.insertexit {
			unsafe {
				(*self.evtmain).remove_event(self.exithd);
			}
			self.insertexit = false;
		}
		self.close_timer();
	}

	fn close_timer(&mut self) {
		if self.timerguid != 0 {
			debug_trace!("call remove_timer {}",self.timerguid);
			unsafe {
				(*self.evtmain).remove_timer(self.timerguid);
			}
			self.timerguid = 0;
		}		
	}

	fn new(mills :i32, times :i32,exithd :u64,evmain :*mut EvtMain) -> Result<Self,Box<dyn Error>> {
		Ok(Self {
			timerguid : 0,
			mills : mills,
			cnt : 0,
			exithd : exithd,
			insertexit : false,
			times : times,
			evtmain : evmain,
		})
	}

	fn add_child_timer(&mut self, parent :TimerTest) -> Result<(),Box<dyn Error>> {
		if !self.insertexit {
			unsafe {
				(*self.evtmain).add_event(Arc::new(RefCell::new(parent.clone())),self.exithd,READ_EVENT)?;
			}
			self.insertexit = true;
		}

		if self.timerguid == 0 {
			unsafe {
				self.timerguid = (*self.evtmain).add_timer(Arc::new(RefCell::new(parent)),self.mills,false)?;
			}
		}
		Ok(())
	}

	fn handle_timer(&mut self, timerguid :u64 , parent :TimerTest) -> Result<(),Box<dyn Error>> {
		debug_trace!("timerguid {} s1.timerguid {}",timerguid,self.timerguid);
		if self.timerguid == timerguid {
			debug_trace!("remove_timer {}",self.timerguid);
			unsafe {
				(*self.evtmain).remove_timer(self.timerguid);
			}
			self.timerguid = 0;
			self.cnt += 1;
			debug_trace!("cnt {} times {}",self.cnt,self.times);
			if self.cnt < self.times {
				unsafe {
					self.timerguid = (*self.evtmain).add_timer(Arc::new(RefCell::new(parent)),self.mills,false)?;	
				}
				
				debug_trace!("add {} new timer",self.timerguid);
			} else {
				debug_trace!("end of timer");
				unsafe {
					(*self.evtmain).break_up()?;
				}
			}
		}
		Ok(())
	}

	fn handle_event(&mut self, evthd :u64,_evttype :u32, _parent :TimerTest) -> Result<(),Box<dyn Error>> {
		debug_trace!("evthd {} handle",evthd);
		if evthd == self.exithd {
			debug_error!("exit notify event");
			unsafe {
				(*self.evtmain).break_up()?;
			}
		} else {
			debug_error!("not accept ");
		}
		Ok(())
	}
}

impl Drop for TimerTest {
	fn drop(&mut self) {
		self.close();
	}
}

impl TimerTest {
	fn close(&mut self) {
		debug_trace!("close TimerTest");
		self.close_inner();
	}

	fn close_inner(&mut self) {
		return;
	}


	fn new(mills :i32, times :i32,exithd :u64, evmain :*mut EvtMain) -> Result<Self,Box<dyn Error>> {
		let retv :Self = Self {
			inner : Arc::new(UnsafeCell::new(TimerTestInner::new(mills,times,exithd,evmain)?)),
		};

		let s1 :&mut TimerTestInner = unsafe {&mut *retv.inner.get()};
		s1.add_child_timer(retv.clone())?;
		Ok(retv)
	}
}

impl EvtCall for TimerTest {
	fn debug_mode(&mut self, _f :&str ,_l :u32) {
		return;
	}

	fn handle(&mut self,evthd :u64, _evttype :u32,_evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>> {
		let s1 :&mut TimerTestInner = unsafe {&mut *self.inner.get()};
		return s1.handle_event(evthd,_evttype,self.clone());
	}

	fn close_event(&mut self,_evthd :u64, _evttype :u32,_evtmain :&mut EvtMain) {
		self.close_inner();
	}
}

impl EvtTimer for TimerTest {
	fn timer(&mut self,timerguid :u64,_evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>> {
		let s1 :&mut TimerTestInner = unsafe {&mut *self.inner.get()};
		return s1.handle_timer(timerguid,self.clone());
	}

	fn close_timer(&mut self,_timerguid :u64, _evtmain :&mut EvtMain) {
		let s1 :&mut TimerTestInner = unsafe {&mut *self.inner.get()};
		s1.close_timer();
	}	
}

#[allow(unused_variables)]
fn timertest_handler(ns :NameSpaceEx,_optargset :Option<Arc<RefCell<dyn ArgSetImpl>>>,_ctx :Option<Arc<RefCell<dyn Any>>>) -> Result<(),Box<dyn Error>> {
	let mut evtmain :EvtMain = EvtMain::new(0)?;
	let sarr :Vec<String>;
	let exithd :u64;
	let mut times :i32 = 1;
	let mut mills :i32 = 200;
	init_log(ns.clone())?;
	sarr = ns.get_array("subnargs");
	if sarr.len() > 0 {
		mills = parse_u64(&sarr[0])? as i32;
	}
	if sarr.len() > 1 {
		times = parse_u64(&sarr[1])? as i32;
	}

	let sigv :Vec<u32> = vec![SIG_INT,SIG_TERM];
	exithd = init_exit_handle(sigv)?;
	let mut timer = TimerTest::new(mills,times,exithd,&mut evtmain)?;
	let _ = evtmain.main_loop()?;	
	timer.close();
	evtmain.close();
	fini_exit_handle();
	Ok(())
}



#[extargs_map_function(timertest_handler)]
pub fn load_timertst_handler(parser :ExtArgsParser) -> Result<(),Box<dyn Error>> {
	let cmdline :String= format!(r#"
		{{
			"timertest<timertest_handler>##[timermills] [timercnt] to test timer##" : {{
				"$" : "*"
			}}
		}}
		"#);
	extargs_load_commandline!(parser,&cmdline)?;
	Ok(())
}