
use std::error::Error;
use std::sync::atomic::{AtomicU16,AtomicU64,Ordering};

use extargsparse_worker::{extargs_error_class,extargs_new_error};

use evtcall::consts::*;
use evtcall::eventfd::*;


extargs_error_class!{SigHdlError}

//static HANDLE_EXIT :AtomicU64 = AtomicU64::new(INVALID_EVENT_HANDLE);
//static ST_SIGINITED :AtomicU16 = AtomicU16::new(0);

static EXIT_EVENTFD :Option<EventFd> = None;


fn rust_signal(_iv :libc::c_int) {
	if EXIT_EVENTFD.is_some() {
		let r :EventFd = EXIT_EVENTFD.as_ref().unwrap().clone();
		let _ = r.set_event();
	}
	return;
}

unsafe extern "system" fn notice_signal(iv : libc::c_int)  {
	rust_signal(iv);
	return;
}

fn get_notice_signal() -> libc::sighandler_t {
	notice_signal as *mut libc::c_void as libc::sighandler_t
}


pub fn init_exit_handle() -> Result<u64,Box<dyn Error>> {
	let mut retv :u64 = INVALID_EVENT_HANDLE;
	if EXIT_EVENTFD.is_none() {
		let cb :EventFd = EventFd::new(0,"exit event")?;	
		EXIT_EVENTFD = Some(cb.clone());
	}
	let b = EXIT_EVENTFD.as_ref().unwrap().clone();
	retv = b.get_event();
	Ok(retv)
}

pub fn fini_exit_handle() {
	if EXIT_EVENTFD.is_some(){
		EXIT_EVENTFD = None;
	}
	return;
}
