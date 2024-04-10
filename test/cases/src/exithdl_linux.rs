
use std::error::Error;
use extargsparse_worker::{extargs_error_class,extargs_new_error};
use evtcall::eventfd::*;

use lazy_static::lazy_static;

extargs_error_class!{SigHdlError}

//static HANDLE_EXIT :AtomicU64 = AtomicU64::new(INVALID_EVENT_HANDLE);
//static ST_SIGINITED :AtomicU16 = AtomicU16::new(0);


lazy_static !{
	static ref EXIT_EVENTFD :Option<EventFd> = _get_exit_fd();
}


fn rust_signal(_iv :libc::c_int) {
	if  EXIT_EVENTFD.is_some() {
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


fn _get_exit_fd() -> Option<EventFd> {
	let bres  = EventFd::new(0,0,"exit event");
	if bres.is_err() {
		return None;
	}

	let sigret :libc::sighandler_t;
	unsafe {
		sigret = libc::signal(libc::SIGINT,get_notice_signal());
	}
	if sigret == libc::SIG_ERR {
		return None;
	}
	Some(bres.unwrap())
}


pub fn init_exit_handle() -> Result<u64,Box<dyn Error>> {
	let retv :u64;
	if EXIT_EVENTFD.is_some() {
		let b =  EXIT_EVENTFD.as_ref().unwrap().clone();
		retv = b.get_event();
	} else {
		extargs_new_error!{SigHdlError,"not init exit event"}
	}
	Ok(retv)
}

pub fn fini_exit_handle() {
	return;
}
