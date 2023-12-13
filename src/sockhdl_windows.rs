
use winapi::um::winnt::{HANDLE};
use winapi::um::winsock2::{WSAStartup,WSADATA,WSADESCRIPTION_LEN,WSASYS_STATUS_LEN,WSACleanup};
use winapi::um::ioapiset::{CancelIoEx};
use winapi::um::errhandlingapi::{GetLastError};
use winapi::um::minwinbase::{OVERLAPPED};
use winapi::shared::minwindef::{MAKEWORD,WORD,LOBYTE,HIBYTE,BOOL,DWORD};
use winapi::ctypes::{c_int};

use super::{evtcall_error_class,evtcall_new_error};
use std::error::Error;
use crate::logger::*;
use crate::*;

evtcall_error_class!{SockHandleError}

#[allow(dead_code)]
enum SockType {
	SockNoneType,
	SockClientType,
	SockServerType,
}

#[allow(dead_code)]
pub struct SockHandle {
	mtype : SockType,
	inconn : i32,
	sock : HANDLE,
	connov :OVERLAPPED,
	inrd :i32,
	rdov :OVERLAPPED,
	inwr :i32,
	wrov :OVERLAPPED,
	inacc :i32,
	accov :OVERLAPPED,
}

impl Drop for SockHandle {
	fn drop(&mut self) {
		self.free();
	}
}

impl SockHandle {
	pub fn free(&mut self) {
		let mut bret :BOOL;
		let mut errval :DWORD;
		match self.mtype {
			SockType::SockClientType => {
				if self.inconn > 0 {
					unsafe {
						bret = CancelIoEx(self.sock,&mut self.connov);
					}
					if bret == 0 {
						unsafe {
							errval = GetLastError();
						}
						evtcall_log_error!("can not CancelIoEx connov {}",errval);
					}
				}
				self.inconn = 0;
			},
			SockType::SockServerType => {
				if self.inacc > 0 {
					unsafe {
						bret = CancelIoEx(self.sock,&mut self.accov);
					}
					if bret == 0 {
						unsafe {
							errval = GetLastError();
						}
						evtcall_log_error!("cannot CancelIoEx accov error {}",errval);
					}
				}
				self.inacc = 0;
			},
			SockType::SockNoneType => {

			},
		}

		if self.inrd > 0 {
			unsafe {
				bret = CancelIoEx(self.sock,&mut self.rdov);
			}
			if bret == 0 {
				unsafe {
					errval = GetLastError();
				}
				evtcall_log_error!("cannot CancelIoEx rdov error {}",errval);
			}
		}
		self.inrd = 0;

		if self.inwr > 0 {
			unsafe {
				bret = CancelIoEx(self.sock,&mut self.wrov);
			}
			if bret == 0 {
				unsafe {
					errval = GetLastError();
				}
				evtcall_log_error!("cannot CancelIoEx wrov error {}",errval);
			}
		}
		self.inwr = 0;


		return;
	}
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

