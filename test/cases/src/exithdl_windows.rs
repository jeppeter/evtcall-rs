
use evtcall::eventfd::*;
use extargsparse_worker::{extargs_error_class,extargs_new_error};
use super::{format_str_log,debug_trace};
use super::loglib::{log_get_timestamp,log_output_function};

use winapi::shared::minwindef::{BOOL,TRUE,FALSE};
use winapi::um::wincon::CTRL_C_EVENT;
use winapi::um::consoleapi::SetConsoleCtrlHandler;
use winapi::um::errhandlingapi::GetLastError;


use std::error::Error;
use lazy_static::lazy_static;

extargs_error_class!{ExitHandleError}



lazy_static !{
	static ref EXIT_EVENTFD :Option<EventFd> = _get_exit_fd();
}


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
	if ty == CTRL_C_EVENT {
		if EXIT_EVENTFD.is_some() {
			let b :EventFd = EXIT_EVENTFD.as_ref().unwrap().clone();
			let _ = b.set_event();
			debug_trace!("set HANDLE_EXIT ");
		}
	}
	return TRUE;
}


fn _get_exit_fd() -> Option<EventFd> {
		let bres  = EventFd::new(0,"exit event");
		let bret :BOOL;
		if bres.is_err() {
			return None;
		}
		unsafe {
			bret = SetConsoleCtrlHandler(Some(ctrl_c_handler),TRUE);
		}
		if bret == FALSE {
			return None;
		}
		Some(bres.unwrap())
}


pub fn init_exit_handle() -> Result<u64,Box<dyn Error>> {
	let retv :u64;
	if EXIT_EVENTFD.is_some() {
		let b = EXIT_EVENTFD.as_ref().unwrap().clone();
		retv = b.get_event();
	} else {
		extargs_new_error!{ExitHandleError,"not init EXIT_EVENTFD {}",get_errno!()}
	}
	return Ok(retv);
}


pub fn fini_exit_handle() {
	return;
}