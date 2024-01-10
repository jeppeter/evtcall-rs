
use std::error::Error;
use std::sync::atomic::{AtomicU16,AtomicU64,Ordering};

use extargsparse_worker::{extargs_error_class,extargs_new_error};

use evtcall::consts::*;


extargs_error_class!{SigHdlError}

static HANDLE_EXIT :AtomicU64 = AtomicU64::new(INVALID_EVENT_HANDLE);
static ST_SIGINITED :AtomicU16 = AtomicU16::new(0);


fn rust_signal(_iv :libc::c_int) {
	if HANDLE_EXIT.load(Ordering::SeqCst) != INVALID_EVENT_HANDLE {
		let cv :libc::eventfd_t = 1;
		unsafe {
			let _fd = HANDLE_EXIT.load(Ordering::SeqCst) as libc::c_int;
			libc::eventfd_write(_fd,cv);
		}
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
	if ST_SIGINITED.load(Ordering::SeqCst) == 0 {
		let mut reti :libc::c_int;
		unsafe {
			reti = libc::eventfd(0,0);
		}
		if reti < 0 {
			extargs_new_error!{SigHdlError,"can not create event fd"}
		}

		HANDLE_EXIT.store(reti as u64, Ordering::SeqCst);
		let sigret :libc::sighandler_t;

		unsafe {
			sigret = libc::signal(libc::SIGINT,get_notice_signal());
		}
		if sigret == libc::SIG_ERR {
			reti = HANDLE_EXIT.load(Ordering::SeqCst) as libc::c_int;
			unsafe {
				libc::close(reti);
			}
			HANDLE_EXIT.store(INVALID_EVENT_HANDLE,Ordering::SeqCst);
			extargs_new_error!{SigHdlError,"cannot set {} signal",libc::SIGINT}
		}
		ST_SIGINITED.store(1,Ordering::SeqCst);
	}
	Ok(HANDLE_EXIT.load(Ordering::SeqCst))
}

pub fn fini_exit_handle() {
	if ST_SIGINITED.load(Ordering::SeqCst) == 1 {
		let reti :libc::c_int;
		reti = HANDLE_EXIT.load(Ordering::SeqCst) as libc::c_int;
		unsafe {
			libc::close(reti);
		}
		HANDLE_EXIT.store(INVALID_EVENT_HANDLE,Ordering::SeqCst);
		ST_SIGINITED.store(0,Ordering::SeqCst);
	}
	return;
}
