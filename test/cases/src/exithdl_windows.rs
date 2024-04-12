
use evtcall::eventfd::*;
use evtcall::consts::*;
use extargsparse_worker::{extargs_error_class,extargs_new_error};
use extlog::{format_str_log,debug_trace};
use extlog::loglib::{log_get_timestamp,log_output_function};

use winapi::shared::minwindef::{BOOL,TRUE,FALSE};
use winapi::um::wincon::{CTRL_C_EVENT,CTRL_BREAK_EVENT};
use winapi::um::consoleapi::SetConsoleCtrlHandler;
use winapi::um::errhandlingapi::GetLastError;


use std::error::Error;
use crate::exithdl_consts::*;

extargs_error_class!{ExitHandleError}


struct CtrlHandleEvent {
	evtfd :Option<EventFd>,
	events : Vec<u32>,
}

//lazy_static !{
static  mut EXIT_EVENTFD :Option<CtrlHandleEvent> = None;
//}


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



unsafe extern "system" fn ctrl_c_handler(ty: u32) -> BOOL {
	debug_trace!("ty 0x{:x}",ty);
	if EXIT_EVENTFD.is_some() {
		let c = EXIT_EVENTFD.as_ref().unwrap();
		for v in c.events.iter() {
			if ty == *v {
				if c.evtfd.is_some() {
					let b :EventFd = c.evtfd.as_ref().unwrap().clone();
					let _ = b.set_event();
					debug_trace!("set 0x{:x} event", *v);
				}
			}
		}

	}
	return TRUE;
}


fn _get_exit_fd(sigv :Vec<u32>) -> Option<CtrlHandleEvent> {
		let mut retv :CtrlHandleEvent = CtrlHandleEvent {
			evtfd : None,
			events : Vec::new(),
		};
		let bres  = EventFd::new(0,0,"exit event");
		let bret :BOOL;
		if bres.is_err() {
			return None;
		}
		retv.evtfd = Some(bres.unwrap());

		for v in sigv {
			let cvv = _trans_exit_value(v);
			if cvv != SIG_VALERR {
				retv.events.push(cvv);
			}
		}

		unsafe {
			bret = SetConsoleCtrlHandler(Some(ctrl_c_handler),TRUE);
		}
		if bret == FALSE {
			return None;
		}


		Some(retv)
}

fn _trans_exit_value(sigv :u32) -> u32 {
	let mut retv : u32 = SIG_VALERR;

	if sigv == SIG_INT {
		retv = CTRL_C_EVENT ;
	} else if sigv == SIG_TERM {
		retv = CTRL_BREAK_EVENT;
	}

	return retv;
}



pub fn init_exit_handle(sigv :Vec<u32>) -> Result<u64,Box<dyn Error>> {
	let mut retv :u64 = INVALID_EVENT_HANDLE;
	let mut ok :bool = false;
	unsafe {
		EXIT_EVENTFD = _get_exit_fd(sigv);
		if EXIT_EVENTFD.is_some() {
			ok = true;
			retv = EXIT_EVENTFD.as_ref().unwrap().evtfd.as_ref().unwrap().get_event();
		}		
	}
	if !ok {
		extargs_new_error!{ExitHandleError,"not init EXIT_EVENTFD {}",get_errno!()}
	}
	return Ok(retv);
}


pub fn fini_exit_handle() {
	return;
}