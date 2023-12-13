
use winapi::um::winnt::{HANDLE,PVOID,LPCWSTR};
//use winapi::um::winsock2::{WSAStartup,WSADATA,WSADESCRIPTION_LEN,WSASYS_STATUS_LEN,WSACleanup,socket,SOCKET,closesocket,INVALID_SOCKET,SOCKET_ERROR,SOCK_STREAM,PF_INET,ioctlsocket,u_long};
use winapi::um::winsock2::*;
use winapi::um::mswsock::{LPFN_CONNECTEX,WSAID_CONNECTEX};
use winapi::shared::ws2def::*;
use winapi::shared::inaddr::*;
use winapi::shared::guiddef::{GUID};
use winapi::um::ws2tcpip::*;
use winapi::um::synchapi::*;
use winapi::um::ioapiset::{CancelIoEx};
use winapi::um::errhandlingapi::{GetLastError,SetLastError};
use winapi::um::minwinbase::{OVERLAPPED,LPSECURITY_ATTRIBUTES,SECURITY_ATTRIBUTES,LPOVERLAPPED};
use winapi::um::handleapi::{CloseHandle};
use winapi::shared::minwindef::{MAKEWORD,WORD,LOBYTE,HIBYTE,BOOL,DWORD,TRUE,FALSE,LPVOID,LPDWORD};
use winapi::ctypes::{c_int,c_void};

use super::{evtcall_error_class,evtcall_new_error};
use super::consts_windows::{NULL_HANDLE_VALUE};
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
	sock : SOCKET,
	connov :OVERLAPPED,
	inrd :i32,
	rdov :OVERLAPPED,
	inwr :i32,
	wrov :OVERLAPPED,
	inacc :i32,
	accov :OVERLAPPED,
	acceptfunc : LPFN_CONNECTEX,
}

impl Drop for SockHandle {
	fn drop(&mut self) {
		self.free();
	}
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

macro_rules! set_errno {
	($val :expr) => {
		unsafe {
			SetLastError($val as DWORD);
		}
	};
}

macro_rules! close_handle_safe {
	($hdval : expr,$name :expr) => {
		let _bret :BOOL;
		let _errval :i32;
		if $hdval != NULL_HANDLE_VALUE {
			unsafe {
				_bret = CloseHandle($hdval);
			}
			if _bret == FALSE {
				_errval = get_errno!();
				evtcall_log_error!("CloseHandle {} error {}",$name,_errval);
			}
		}
		$hdval = NULL_HANDLE_VALUE;
	};
}

macro_rules! close_socket_safe {
	($sockval :expr , $name :expr) => {
		let _errval :i32;
		let _iret :c_int;

		if $sockval != INVALID_SOCKET {
			unsafe {
				_iret = closesocket($sockval);
			}

			if _iret == SOCKET_ERROR {
				_errval = get_errno!();
				evtcall_log_error!("close {} error {}",$name,_errval);
			}
		}
		$sockval = INVALID_SOCKET;
	};
}

macro_rules! new_ov {
	() => { {
		let c :OVERLAPPED = unsafe {std::mem::zeroed()};
		c
	}};
}

macro_rules! create_event_safe {
	($hd :expr,$freval :expr,$name :expr) => {
		let _errval :i32;
		let _pattr :LPSECURITY_ATTRIBUTES = std::ptr::null_mut::<SECURITY_ATTRIBUTES>() as LPSECURITY_ATTRIBUTES;
		let _pstr :LPCWSTR = std::ptr::null() as LPCWSTR;
		$hd = unsafe {CreateEventW(_pattr,TRUE,FALSE,_pstr)};
		if $hd == NULL_HANDLE_VALUE {
			_errval = get_errno!();
			$freval.free();
			evtcall_new_error!{SockHandleError,"create {} error {}",$name,_errval}
		}
	};
}

macro_rules! cancel_io_safe {
	($val :expr,$hdval :expr, $ovval :expr , $name :expr) => {
		let _bret :BOOL;
		let _errval :DWORD;
		if $val > 0 {
			unsafe {
				_bret = CancelIoEx($hdval as HANDLE,&mut $ovval);
			}
			if _bret == 0 {
				unsafe {
					_errval = GetLastError();
				}
				evtcall_log_error!("CancelIoEx {} error {}",$name,_errval);
			}
		}
		$val = 0;
	};
}

impl SockHandle {
	pub fn free(&mut self) {
		match self.mtype {
			SockType::SockClientType => {
				cancel_io_safe!(self.inconn,self.sock,self.connov,"connov");
				close_handle_safe!(self.connov.hEvent,"connov handle");
			},
			SockType::SockServerType => {
				cancel_io_safe!(self.inacc,self.sock,self.accov,"accov");
				close_handle_safe!(self.accov.hEvent,"accov handle");
			},
			SockType::SockNoneType => {

			},
		}

		cancel_io_safe!(self.inrd,self.sock,self.rdov,"rdov");
		close_handle_safe!(self.rdov.hEvent,"rdov handle");

		cancel_io_safe!(self.inwr,self.sock,self.wrov,"wrov");
		close_handle_safe!(self.wrov.hEvent,"wrov handle");

		close_socket_safe!(self.sock,"sock handle");

		return;
	}

	#[allow(dead_code)]
	#[allow(unused_variables)]
	#[allow(unused_mut)]
	pub fn bind_server(ipaddr :&str,port :u32,localip :&str,localport :u32,connected :bool) -> Result<Self,Box<dyn Error>> {
		if ipaddr.len() == 0 || port == 0 {
			evtcall_new_error!{SockHandleError,"not valid ipaddr [{}] or port [{}]",ipaddr,port}
		}
		let mut retv :Self = Self {
			mtype : SockType::SockServerType,
			sock : INVALID_SOCKET,
			inconn : 0,
			connov : new_ov!(),
			inacc : 0,
			accov : new_ov!(),
			inrd : 0,
			rdov : new_ov!(),
			inwr : 0,
			wrov : new_ov!(),
			acceptfunc : None,
		};
		let mut ret :i32;
		let mut iret :c_int;
		let mut block :u_long;
		let mut name :SOCKADDR_IN = unsafe {std::mem::zeroed()};
		let namelen :c_int;
		let accguid:GUID = WSAID_CONNECTEX;
		let mut dret :DWORD = 0;
		let mut bret :BOOL;



		unsafe {
			retv.sock = socket(AF_INET,winapi::shared::ws2def::SOCK_STREAM,0);
		}

		if retv.sock == INVALID_SOCKET {
			ret = get_errno!();
			retv.free();
			evtcall_new_error!{SockHandleError,"cannot socket error {}",ret}
		}

		block = 1;
		unsafe {
			iret = ioctlsocket(retv.sock,FIONBIO,&mut block);
		}

		if iret == SOCKET_ERROR {
			ret = get_errno!();
			retv.free();
			evtcall_new_error!{SockHandleError,"cannot set non-block error {}",ret}
		}

		name.sin_family = AF_INET as u16;
		if localip.len() != 0 {
			let pv : PVOID = ((&mut name.sin_addr) as * mut IN_ADDR) as PVOID;
			unsafe {
				inet_pton(AF_INET,localip.as_bytes().as_ptr() as * const i8,pv);
			}
		} else {
			name.sin_addr = unsafe {std::mem::zeroed()};
		}

		if localport != 0 {
			name.sin_port = unsafe {htons(localport as u16)};	
		} else {
			name.sin_port = 0;
		}

		namelen = std::mem::size_of::<SOCKADDR_IN>() as c_int;
		unsafe {
			let pv :*const SOCKADDR = (&name as *const SOCKADDR_IN) as *const SOCKADDR;
			iret = bind(retv.sock,pv, namelen);
		}

		if iret == SOCKET_ERROR {
			ret = get_errno!();
			retv.free();
			evtcall_new_error!{SockHandleError,"bind [{}:{}] error {}",localip,localport,ret}
		}

		create_event_safe!(retv.connov.hEvent,retv,"connov handle");

		name = unsafe {std::mem::zeroed()};
		name.sin_family = AF_INET as u16;
		{
			let pv : PVOID = ((&mut name.sin_addr) as * mut IN_ADDR) as PVOID;
			unsafe {
				inet_pton(AF_INET,ipaddr.as_bytes().as_ptr() as * const i8,pv);
			}
		}
		name.sin_port = unsafe {htons(port as u16)};

		unsafe {
			let _funcptr :LPVOID = (&mut retv.acceptfunc as *mut LPFN_CONNECTEX ) as LPVOID;
			let _funcsize:DWORD = std::mem::size_of::<LPFN_CONNECTEX>() as DWORD;
			let _guidptr :LPVOID = (&accguid as *const GUID) as LPVOID;
			let _guidsize :DWORD = std::mem::size_of::<GUID>() as DWORD;
			let _retptr :LPDWORD = &mut dret;
			let _ptrov :LPWSAOVERLAPPED = std::ptr::null::<WSAOVERLAPPED>() as LPWSAOVERLAPPED;
			let _fncall :LPWSAOVERLAPPED_COMPLETION_ROUTINE = None;
			iret = WSAIoctl(retv.sock,SIO_GET_EXTENSION_FUNCTION_POINTER,_guidptr,_guidsize,_funcptr,_funcsize,_retptr,_ptrov,_fncall);
		}

		if iret != 0 {
			iret = get_errno!();
			retv.free();
			evtcall_new_error!{SockHandleError,"cannot get WSAIoctl error {}",iret}
		}

		if retv.acceptfunc.is_none() {
			retv.free();
			evtcall_new_error!{SockHandleError,"cannot get acceptfunc"}
		}
		{
			let _nameptr :*const SOCKADDR = (&name as *const SOCKADDR_IN) as * const SOCKADDR;
			let _namelen :c_int = std::mem::size_of::<SOCKADDR_IN>() as c_int;
			let _sendbuf :LPVOID = std::ptr::null_mut::<c_void>();
			let _sndsize :DWORD = 0;
			let _retsend :LPDWORD = &mut dret;
			let _connovptr :LPOVERLAPPED = &mut retv.connov;
			unsafe {
				bret = retv.acceptfunc.as_ref().unwrap()(retv.sock,_nameptr,_namelen,_sendbuf,_sndsize,_retsend,_connovptr);	
			}
			if bret == FALSE {
				iret = get_errno!();
				retv.free();
				evtcall_new_error!{SockHandleError,"call acceptfunc error {}",iret}
			}
		}

		


		Ok(retv)
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

