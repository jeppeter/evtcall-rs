use extargsparse_codegen::{extargs_load_commandline,extargs_map_function};
//use extargsparse_worker::{extargs_error_class,extargs_new_error};
use extargsparse_worker::namespace::{NameSpaceEx};
use extargsparse_worker::argset::{ArgSetImpl};
use extargsparse_worker::parser::{ExtArgsParser};
use extargsparse_worker::funccall::{ExtArgsParseFunc};


use std::cell::RefCell;
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
use extargsparse_worker::{extargs_error_class,extargs_new_error};


use super::loglib::{log_get_timestamp,log_output_function,init_log};
use super::strop::*;
use super::*;
use evtcall::eventfd::*;
use rand::prelude::*;

extargs_error_class!{NetHdlError}

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


#[extargs_map_function(logtstthr_handler)]
pub fn load_thread_handler(parser :ExtArgsParser) -> Result<(),Box<dyn Error>> {
	let cmdline = r#"
	{
		"logtstthr<logtstthr_handler>##[times] [threads] to log##" : {
			"$" : "*"
		}
	}
	"#;
	extargs_load_commandline!(parser,cmdline)?;
	Ok(())
}