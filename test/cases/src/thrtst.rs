use extargsparse_codegen::{extargs_load_commandline,extargs_map_function};
//use extargsparse_worker::{extargs_error_class,extargs_new_error};
use extargsparse_worker::namespace::{NameSpaceEx};
use extargsparse_worker::argset::{ArgSetImpl};
use extargsparse_worker::parser::{ExtArgsParser};
use extargsparse_worker::funccall::{ExtArgsParseFunc};


use std::cell::RefCell;
use std::sync::Arc;
use std::sync::mpsc;
use std::error::Error;
use std::boxed::Box;
#[allow(unused_imports)]
use regex::Regex;
#[allow(unused_imports)]
use std::any::Any;

use lazy_static::lazy_static;
use std::collections::HashMap;

#[allow(unused_imports)]
use extargsparse_worker::{extargs_error_class,extargs_new_error};


use super::loglib::{log_get_timestamp,log_output_function,init_log};
use super::strop::*;
use super::*;
use evtcall::eventfd::*;
use rand::prelude::*;

extargs_error_class!{ThrHdlError}

#[allow(non_camel_case_types)]
struct logarg {
	num :i32,
	stopsig :Arc<EventFd>,
}


fn logtest_thread(arg :logarg) {
	let mut rnd = rand::thread_rng();
	for i in 0..arg.num {
		debug_trace!("{:?} thread {} trace",std::thread::current().id(),i);
		debug_debug!("{:?} thread {} debug",std::thread::current().id(),i);
		debug_info!("{:?} thread {} info",std::thread::current().id(),i);
		debug_warn!("{:?} thread {} warn",std::thread::current().id(),i);
		debug_error!("{:?} thread {} error",std::thread::current().id(),i);
		let val :u64 = rnd.gen::<u64>() % 1000;
		debug_error!("{:?} sleep {}",std::thread::current().id(),val);
		std::thread::sleep(std::time::Duration::from_millis(val));
	}
	debug_error!("{:?} exit [{}]",std::thread::current().id(),arg.stopsig.get_name());
	let _ = arg.stopsig.set_event();
	return;
}

fn logtstthr_handler(ns :NameSpaceEx,_optargset :Option<Arc<RefCell<dyn ArgSetImpl>>>,_ctx :Option<Arc<RefCell<dyn Any>>>) -> Result<(),Box<dyn Error>> {	
	let sarr :Vec<String>  = ns.get_array("subnargs");
	let mut times :i32 = 10;
	let mut thrids :usize = 1;
	let mut handles = Vec::new();
	let mut stopvec :Vec<Arc<EventFd>>=Vec::new();
	let mut namevec :Vec<String> = Vec::new();
	let mut bname :String;
	//let mut rnd = rand::thread_rng();


	init_log(ns.clone())?;
	if sarr.len() > 0 {
		times = parse_u64(&sarr[0])? as i32;
	}
	if sarr.len() > 1 {
		thrids = parse_u64(&sarr[1])? as usize;
	}

	for i in 0..thrids {
		bname = format!("thr event {}",i);
		let curstop :Arc<EventFd> = Arc::new(EventFd::new(0,&bname)?);
		namevec.push(format!("thr event {}",i));
		stopvec.push(curstop.clone());
		let logvar = logarg {
			num : times,
			stopsig : curstop.clone(),
		};
		handles.push(std::thread::spawn(move || {
			logtest_thread(logvar);
		}));
	}

	for i in 0..times {
		debug_trace!("main thread {} trace",i);
		debug_debug!("main thread {} debug",i);
		debug_info!("main thread {} info",i);
		debug_warn!("main thread {} warn",i);
		debug_error!("main thread {} error",i);
		//let val :u64 = rnd.gen::<u64>() % 1000;
		//debug_error!("main sleep {}",val);
		//std::thread::sleep(std::time::Duration::from_millis(val));
	}

	loop {
		if stopvec.len() == 0 {
			break;
		}
		let mut fidx :i32 = -1;
		for i in 0..stopvec.len() {
			let bval = stopvec[i].is_event()?;
			if bval {
				fidx = i as i32;
				debug_error!("get {} exitnotice",fidx);
				break;
			}
		}

		if fidx >= 0 {
			debug_error!("remove [{}]thread [{}]",fidx,namevec[fidx as usize]);
			stopvec.remove(fidx as usize);
			namevec.remove(fidx as usize);
			let h = handles.remove(fidx as usize);
			h.join().unwrap();
		}

		if stopvec.len() > 0 {
			std::thread::sleep(std::time::Duration::from_millis(1 * 100));
		}
	}

	Ok(())
}

fn logchannel_thread(arg :logarg,rx :mpsc::Receiver<i32>) {
	let mut rnd = rand::thread_rng();
	for i in 0..arg.num {
		debug_error!("{:?} thread {} error",std::thread::current().id(),i);
		loop {
			let cres = rx.try_recv();
			if cres.is_ok() {
				debug_error!("{:?} recv {}",std::thread::current().id(),cres.unwrap());
			} else {
				break;
			}
		}
		let val :u64 = rnd.gen::<u64>() % 1000;
		debug_error!("{:?} sleep {}",std::thread::current().id(),val);
		std::thread::sleep(std::time::Duration::from_millis(val));
	}
	debug_error!("{:?} exit [{}]",std::thread::current().id(),arg.stopsig.get_name());
	let _ = arg.stopsig.set_event();
	return;
}


fn thrsharedata_handler(ns :NameSpaceEx,_optargset :Option<Arc<RefCell<dyn ArgSetImpl>>>,_ctx :Option<Arc<RefCell<dyn Any>>>) -> Result<(),Box<dyn Error>> {	
	let sarr :Vec<String>  = ns.get_array("subnargs");
	let mut times :i32 = 10;
	let mut thrids :usize = 1;
	let mut handles = Vec::new();
	let mut stopvec :Vec<Arc<EventFd>>=Vec::new();
	let mut namevec :Vec<String> = Vec::new();
	let mut sndchannels :Vec<mpsc::Sender<i32>> = Vec::new();
	let mut bname :String;
	//let mut rnd = rand::thread_rng();


	init_log(ns.clone())?;
	if sarr.len() > 0 {
		times = parse_u64(&sarr[0])? as i32;
	}
	if sarr.len() > 1 {
		thrids = parse_u64(&sarr[1])? as usize;
	}

	for i in 0..thrids {
		bname = format!("thr event {}",i);
		let curstop :Arc<EventFd> = Arc::new(EventFd::new(0,&bname)?);
		let (tx,rx) = mpsc::channel::<i32>();
		namevec.push(format!("thr event {}",i));
		stopvec.push(curstop.clone());
		sndchannels.push(tx.clone());
		let logvar = logarg {
			num : times,
			stopsig : curstop.clone(),
		};
		handles.push(std::thread::spawn(move || {
			logchannel_thread(logvar,rx);
		}));
	}

	for i in 0..times {
		let mut jdx :usize = 0;
		while jdx < sndchannels.len() {
			let _ = sndchannels[jdx].send(i);
			jdx += 1;
		}
	}

	loop {
		if stopvec.len() == 0 {
			break;
		}
		let mut fidx :i32 = -1;
		for i in 0..stopvec.len() {
			let bval = stopvec[i].is_event()?;
			if bval {
				fidx = i as i32;
				debug_error!("get {} exitnotice",fidx);
				break;
			}
		}

		if fidx >= 0 {
			debug_error!("remove [{}]thread [{}]",fidx,namevec[fidx as usize]);
			stopvec.remove(fidx as usize);
			namevec.remove(fidx as usize);
			sndchannels.remove(fidx as usize);
			let h = handles.remove(fidx as usize);
			h.join().unwrap();
		}

		if stopvec.len() > 0 {
			std::thread::sleep(std::time::Duration::from_millis(1 * 100));
		}
	}

	Ok(())
}

struct CommonChannelInner {
	thrrcv : EvtChannel<String>,
	thrsnd : EvtChannel<String>,
	exitevt : EventFd,
	exitnotify : EventFd,
	evtmain :*mut EvtMain,	
	insertrcv : bool,
	insertexit : bool,
}

#[derive(Clone)]
struct CommonChannel  {
	inner :Arc<RefCell<CommonChannelInner>>,
}

impl CommonChannelInner {
	fn new(snd :EvtChannel<String>,rcv :EvtChannel<String>, exitevt : EventFd,exitnotify :EventFd,evtmain :*mut EvtMain) -> Result<Arc<RefCell<Self>>, Box<dyn Error>> {
		let retv :Self = Self {
			thrrcv : rcv.clone(),
			thrsnd : snd.clone(),
			exitevt : exitevt.clone(),
			exitnotify : exitnotify.clone(),
			evtmain : evtmain,
			insertrcv : false,
			insertexit : false,
		};

		Ok(Arc::new(RefCell::new(retv)))
	}

	fn add_events(&mut self, parent : CommonChannel) -> Result<(),Box<dyn Error>> {
		if !self.thrrcv {
			let _ = self.evtmain.add_event(Arc::new(RefCell::new(parent.clone())),self.exitevt.get_event(),READ_EVENT)?;
			self.insertrcv = true;

		}
		if !self.insertexit {
			let _ = self.evtmain.add_event(Arc::new(RefCell::new(parent.clone())),self.thrrcv.get_evt(),READ_EVENT)?;
			self.insertexit = true;			
		}
		Ok(())
	}

	fn close(&mut self) {
		self.close_event();
	}

	fn close_event(&mut self) {
		if self.insertrcv {
			self.evtmain.remove_event(self.thrrcv.get_evt());
			self.insertrcv = false;
		}
		if self.insertexit {
			self.evtmain.remove_event(self.exitevt.get_event());
			self.insertexit = false;
		}
		return;		
	}

	fn handle_event(&mut self, evthd : u64, _evttype : u32,parent :CommonChannel) -> Result<(),Box<dyn Error>> {
		if evthd == self.thrrcv.get_evt() {

		} else if evthd == self.exitevt.get_event() {

		} else {
			extargs_new_error!{ThrHdlError,"thread [{:?}]not support evthd 0x{:x}",std::thread::current().id(),evthd}
		}
		Ok(())
	}

}

impl CommonChannel {
	fn new(rcv :EvtChannel<String>,snd :EvtChannel<String>,exitevt : EventFd,exitnotify :EventFd, evtmain :*mut EvtMain) -> Result<Self,Box<dyn Error>> {
		let mut retv : Self = Self {
			inner : CommonChannelInner::new(rcv,snd,exitevt,exitnotify,evtmain)?,
		};
		let _ = retv.inner.borrow_mut().add_events(retv.clone())?;
		Ok(retv)
	}

}

impl EvtCall for CommonChannel {
	fn debug_mode(&mut self,_fname :&str, _lineno :u32) {
		return;
	}

	fn handle(&mut self,evthd :u64, _evttype :u32,_evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>> {
		return self.inner.borrow_mut().handle_event(evthd,_evttype,self.clone());
	}
}

fn logchannel_thread(chl :CommonChannel) -> Result<(),Box<dyn Error>> {
	let mut evtmain :EvtMain;

	evtmain = EvtMain::new(0)?;


	return;
}


#[extargs_map_function(logtstthr_handler,thrsharedata_handler)]
pub fn load_thread_handler(parser :ExtArgsParser) -> Result<(),Box<dyn Error>> {
	let cmdline = r#"
	{
		"logtstthr<logtstthr_handler>##[times] [threads] to log##" : {
			"$" : "*"
		},
		"thrsharedata<thrsharedata_handler>##[times] [threads] to share data##" : {
			"$" : "*"
		},
		"thrchannel<thrchannel_handler>##[times] [threads] to communicate channel##" : {
			"$" : "*"
		}
	}
	"#;
	extargs_load_commandline!(parser,cmdline)?;
	Ok(())
}