
use evtcall::eventfd::*;
use extargsparse_worker::{extargs_error_class,extargs_new_error};
use super::{format_str_log,debug_trace};
use super::loglib::{log_get_timestamp,log_output_function};

use winapi::shared::minwindef::{BOOL,TRUE,FALSE};
use winapi::um::wincon::CTRL_C_EVENT;
use winapi::um::consoleapi::SetConsoleCtrlHandler;
use winapi::um::errhandlingapi::GetLastError;


use std::error::Error;

extargs_error_class!{ExitHandleError}

//static mut HANDLE_EXIT :u64 = INVALID_EVENT_HANDLE;
static mut EXIT_EVENTFD :Option<EventFd> = None;

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


pub fn init_exit_handle() -> Result<u64,Box<dyn Error>> {
	let retv :u64 ;
	let bret :BOOL;
	if unsafe {EXIT_EVENTFD.is_none()} {
		let b :EventFd = EventFd::new(0,"exit event")?;
		unsafe {EXIT_EVENTFD =Some(b.clone())};
		bret = unsafe {
			SetConsoleCtrlHandler(Some(ctrl_c_handler),TRUE)
		};
		if bret == FALSE {
			let reti = get_errno!();
			unsafe{EXIT_EVENTFD = None};
			extargs_new_error!{ExitHandleError,"not insert ctrl_c_handler error {}",reti}
		}
		debug_trace!("HANDLE_EXIT ");
	}
	let b = unsafe {EXIT_EVENTFD.as_ref().unwrap().clone()};
	retv = b.get_event();
	return Ok(retv);
}


pub fn fini_exit_handle() {
	if unsafe {EXIT_EVENTFD.is_some()} {
		unsafe {EXIT_EVENTFD = None};
	}
	return;
}