
use winapi::um::winsock2::{WSAStartup,WSADATA,WSADESCRIPTION_LEN,WSASYS_STATUS_LEN,WSACleanup};
use winapi::shared::minwindef::{MAKEWORD,WORD,LOBYTE,HIBYTE};
use winapi::ctypes::{c_int};

use super::{evtcall_error_class,evtcall_new_error};
use std::error::Error;


evtcall_error_class!{SockHandleError}

#[allow(dead_code)]
pub struct SockHandle {

}

#[allow(dead_code)]
pub fn init_socket() -> Result<(),Box<dyn Error>> {
	let sockver :WORD;
	let mut wsdata :WSADATA = WSADATA{
		wVersion : 0,
		wHighVersion : 0,
		iMaxSockets : 0,
		iMaxUdpDg : 0,
		lpVendorInfo : std::ptr::null_mut(),
		szDescription : [0;WSADESCRIPTION_LEN + 1],
		szSystemStatus : [0;WSASYS_STATUS_LEN + 1],
	};
	let ret :c_int;
	sockver = MAKEWORD(2,2);
	unsafe {
		ret = WSAStartup(sockver,&mut wsdata);	
	}
	
	if ret != 0 {
		unsafe {
			WSACleanup();
		}
		evtcall_new_error!{SockHandleError,"cannot WSAStartup {}",ret}
	}

	if LOBYTE(wsdata.wVersion) != 2 || HIBYTE(wsdata.wVersion) != 2 {
		unsafe {
			WSACleanup();
		}
		evtcall_new_error!{SockHandleError,"wVersion {} not valid",wsdata.wVersion}
	}

	Ok(())
}

#[allow(dead_code)]
pub fn fini_socket()  {
	unsafe {
		WSACleanup();
	}
}