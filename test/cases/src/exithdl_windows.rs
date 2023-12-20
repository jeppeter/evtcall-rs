
use evtcall::consts::*;
use extargsparse_worker::{extargs_error_class,extargs_new_error};
use super::{format_str_log,debug_error,debug_trace};
use super::loglib::{log_get_timestamp,log_output_function};

use winapi::shared::minwindef::{TRUE,FALSE,BOOL};
use winapi::um::winnt::{HANDLE,LPCWSTR};
use winapi::um::wincon::{CTRL_C_EVENT};
use winapi::um::synchapi::{SetEvent,CreateEventW};
use winapi::um::minwinbase::{LPSECURITY_ATTRIBUTES,SECURITY_ATTRIBUTES};
use winapi::um::errhandlingapi::{GetLastError};
use winapi::um::consoleapi::{SetConsoleCtrlHandler};
use winapi::um::handleapi::{CloseHandle};


use std::error::Error;

extargs_error_class!{ExitHandleError}

static mut HANDLE_EXIT :u64 = INVALID_EVENT_HANDLE;

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
		unsafe {$hd = CreateEventW(_pattr,TRUE,FALSE,_pstr) as u64;}
		if unsafe {$hd} == 0 {
			unsafe {$hd = INVALID_EVENT_HANDLE};
			_errval = get_errno!();
			extargs_new_error!{ExitHandleError,"create {} error {}",$name,_errval}
		}
	};
}

unsafe extern "system" fn ctrl_c_handler(ty: u32) -> BOOL {
	debug_trace!("ty 0x{:x}",ty);
	if ty == CTRL_C_EVENT {
		if HANDLE_EXIT != INVALID_EVENT_HANDLE {
			unsafe {
				SetEvent(HANDLE_EXIT as HANDLE);
			}
			debug_trace!("set HANDLE_EXIT");
		}
	}
	return TRUE;
}


pub fn init_exit_handle() -> Result<u64,Box<dyn Error>> {
	let bret :BOOL;
	if unsafe {HANDLE_EXIT} == INVALID_EVENT_HANDLE {
		create_event_safe!(HANDLE_EXIT,"HANDLE_EXIT");
		bret = unsafe {
			SetConsoleCtrlHandler(Some(ctrl_c_handler),TRUE)
		};
		if bret == FALSE {
			let reti = get_errno!();
			unsafe {CloseHandle(HANDLE_EXIT as HANDLE)};
			unsafe{HANDLE_EXIT = INVALID_EVENT_HANDLE};
			extargs_new_error!{ExitHandleError,"not insert ctrl_c_handler error {}",reti}
		}
		debug_trace!("HANDLE_EXIT 0x{:x}",unsafe{HANDLE_EXIT});
	}
	return Ok(unsafe {HANDLE_EXIT});
}


pub fn fini_exit_handle() {
	let bret :BOOL;
	let reti :i32;
	if unsafe {HANDLE_EXIT} != INVALID_EVENT_HANDLE {
		unsafe {
			bret = CloseHandle(HANDLE_EXIT as HANDLE);
		}
		if bret == FALSE {
			reti = get_errno!();
			debug_error!("close HANDLE_EXIT error {}",reti);
		}
	}
	unsafe {HANDLE_EXIT = INVALID_EVENT_HANDLE};
	return;
}