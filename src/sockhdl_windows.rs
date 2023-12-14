
use winapi::um::winnt::{HANDLE,PVOID,LPCWSTR};
use winapi::um::winsock2::{SOL_SOCKET,SO_REUSEADDR,setsockopt,socket,SOCKET,INVALID_SOCKET,closesocket,SOCKET_ERROR,htons,bind,LPWSAOVERLAPPED,WSAOVERLAPPED,LPWSAOVERLAPPED_COMPLETION_ROUTINE,WSAIoctl,WSADATA,WSADESCRIPTION_LEN,WSASYS_STATUS_LEN,WSAStartup,WSACleanup,u_long,ioctlsocket,FIONBIO,WSAGetLastError,listen};
use winapi::um::mswsock::{LPFN_ACCEPTEX,WSAID_ACCEPTEX};
use winapi::shared::ws2def::*;
use winapi::shared::inaddr::*;
use winapi::shared::guiddef::{GUID};
use winapi::um::ws2tcpip::*;
use winapi::um::synchapi::*;
use winapi::um::ioapiset::{CancelIoEx};
use winapi::um::errhandlingapi::{GetLastError,SetLastError};
use winapi::um::minwinbase::{OVERLAPPED,LPSECURITY_ATTRIBUTES,SECURITY_ATTRIBUTES};
use winapi::um::handleapi::{CloseHandle};
use winapi::shared::minwindef::{MAKEWORD,WORD,LOBYTE,HIBYTE,BOOL,DWORD,TRUE,FALSE,LPVOID,LPDWORD};
use winapi::ctypes::{c_int,c_char};

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
	accsock :SOCKET,
	connov :OVERLAPPED,
	inrd :i32,
	rdov :OVERLAPPED,
	inwr :i32,
	wrov :OVERLAPPED,
	inacc :i32,
	accov :OVERLAPPED,
	acceptfunc : LPFN_ACCEPTEX,
	localaddr : String,
	localport : u32,
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

macro_rules! get_wsa_errno {
	() => {{
		let mut retv :i32 ;
		unsafe {
			retv = WSAGetLastError() as i32;
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
		close_socket_safe!(self.accsock,"accsock handle");

		return;
	}

	fn _default_new(socktype :SockType) -> Self {
		Self {
			mtype : socktype,
			sock : INVALID_SOCKET,
			accsock : INVALID_SOCKET,
			inconn : 0,
			connov : new_ov!(),
			inacc : 0,
			accov : new_ov!(),
			inrd : 0,
			rdov : new_ov!(),
			inwr : 0,
			wrov : new_ov!(),
			acceptfunc : None,
			localaddr : format!(""),
			localport : 0,
		}
	}

	fn _bind_addr(&mut self,ipaddr :&str, port :u32) -> Result<(),Box<dyn Error>> {
		let mut name :SOCKADDR_IN = unsafe {std::mem::zeroed()};
		let ret :i32;
		let iret :c_int;
		let namelen :c_int;
		name.sin_family = AF_INET as u16;
		if ipaddr.len() > 0 {
			unsafe {
				let _pv : PVOID = ((&mut name.sin_addr) as * mut IN_ADDR) as PVOID;
				let _addr = ipaddr.as_bytes().as_ptr() as * const i8;
				inet_pton(AF_INET,_addr,_pv);
			}			
		} else {
			name.sin_addr = unsafe {std::mem::zeroed()};
		}
		if port != 0 {
			name.sin_port = unsafe {htons(port as u16)};	
		} else {
			name.sin_port = 0;
		}
		
		namelen = std::mem::size_of::<SOCKADDR_IN>() as c_int;
		unsafe {
			let _pv :*const SOCKADDR = (&name as *const SOCKADDR_IN) as *const SOCKADDR;
			iret = bind(self.sock,_pv, namelen);
		}

		if iret == SOCKET_ERROR {
			ret = get_wsa_errno!();
			evtcall_new_error!{SockHandleError,"bind [{}:{}] error {}",ipaddr,port,ret}
		}
		Ok(())
	}

	fn _get_accept_func(&mut self, guid :&GUID) -> Result<(),Box<dyn Error>> {
		let mut iret :c_int;
		let mut dret :DWORD = 0;
		unsafe {
			let _funcptr :LPVOID = (&mut self.acceptfunc as *mut LPFN_ACCEPTEX ) as LPVOID;
			let _funcsize:DWORD = std::mem::size_of::<LPFN_ACCEPTEX>() as DWORD;
			let _guidptr :LPVOID = (guid as *const GUID) as LPVOID;
			let _guidsize :DWORD = std::mem::size_of::<GUID>() as DWORD;
			let _retptr :LPDWORD = &mut dret;
			let _ptrov :LPWSAOVERLAPPED = std::ptr::null::<WSAOVERLAPPED>() as LPWSAOVERLAPPED;
			let _fncall :LPWSAOVERLAPPED_COMPLETION_ROUTINE = None;
			iret = WSAIoctl(self.sock,SIO_GET_EXTENSION_FUNCTION_POINTER,_guidptr,_guidsize,_funcptr,_funcsize,_retptr,_ptrov,_fncall);
		}

		if iret != 0 {
			iret = get_wsa_errno!();
			evtcall_new_error!{SockHandleError,"cannot get WSAIoctl error {}",iret}
		}
		Ok(())
	}

	fn _inner_accept(&mut self) -> Result<(),Box<dyn Error>> {
		let ret :i32;
		if self.accsock != INVALID_SOCKET {
			evtcall_new_error!{SockHandleError,"accsock not in"}
		}

		unsafe {
			self.accsock = socket(AF_INET,SOCK_STREAM,0);
		}
		if self.accsock == INVALID_SOCKET {
			ret = get_wsa_errno!();
			evtcall_new_error!{SockHandleError,"socket accsock error {}",ret}
		}
		Ok(())
	}

	#[allow(dead_code)]
	#[allow(unused_variables)]
	#[allow(unused_mut)]
	pub fn bind_server(ipaddr :&str,port :u32,backlog : i32) -> Result<Self,Box<dyn Error>> {
		if ipaddr.len() == 0 || port == 0 {
			evtcall_new_error!{SockHandleError,"not valid ipaddr [{}] or port [{}]",ipaddr,port}
		}
		let mut retv :Self = Self::_default_new(SockType::SockServerType);
		let mut ret :i32;
		let mut iret :c_int;
		let mut opt :c_int;
		let mut block :u_long;
		let accguid:GUID = WSAID_ACCEPTEX;
		let mut dret :DWORD = 0;
		let mut bret :BOOL;

		retv.localaddr = format!("{}",ipaddr);
		retv.localport = port;
		unsafe {
			retv.sock = socket(AF_INET,winapi::shared::ws2def::SOCK_STREAM,0);
		}

		if retv.sock == INVALID_SOCKET {
			ret = get_errno!();
			evtcall_new_error!{SockHandleError,"cannot socket error {}",ret}
		}

		opt = 1;
		unsafe {
			let _optptr = (&opt as *const c_int) as *const c_char;
			let _optsize = std::mem::size_of::<DWORD>() as c_int;
			iret = setsockopt(retv.sock,SOL_SOCKET,SO_REUSEADDR,_optptr,_optsize);
		}

		if iret == SOCKET_ERROR {
			ret = get_errno!();
			evtcall_new_error!{SockHandleError,"cannot setopt reuse error {}",ret}
		}


		block = 1;
		unsafe {
			let _blkptr = &mut block as * mut u_long;
			iret = ioctlsocket(retv.sock,FIONBIO,_blkptr);
		}

		if iret == SOCKET_ERROR {
			ret = get_errno!();
			evtcall_new_error!{SockHandleError,"cannot set non-block error {}",ret}
		}

		retv._bind_addr(ipaddr,port)?;

		create_event_safe!(retv.connov.hEvent,retv,"connov handle");

		unsafe {
			iret = listen(retv.sock,backlog);
		}
		if iret == SOCKET_ERROR {
			ret = get_wsa_errno!();
			evtcall_new_error!{SockHandleError,"cannot listen error {}",ret}

		}

		retv._get_accept_func(&accguid)?;
		retv._inner_accept()?;

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

