
use winapi::um::winnt::{HANDLE,PVOID,LPCWSTR,PSTR};
use winapi::um::winsock2::{SOL_SOCKET,SO_REUSEADDR,setsockopt,getsockopt,socket,SOCKET,INVALID_SOCKET,closesocket,SOCKET_ERROR,htons,bind,LPWSAOVERLAPPED,WSAOVERLAPPED,LPWSAOVERLAPPED_COMPLETION_ROUTINE,WSAIoctl,WSADATA,WSADESCRIPTION_LEN,WSASYS_STATUS_LEN,WSAStartup,WSACleanup,u_long,ioctlsocket,FIONBIO,WSAGetLastError,listen,WSA_IO_PENDING,getpeername,ntohs,getsockname,MSG_PARTIAL,WSARecv,WSASend};
use winapi::um::mswsock::{LPFN_ACCEPTEX,WSAID_ACCEPTEX,SO_UPDATE_ACCEPT_CONTEXT,LPFN_CONNECTEX,WSAID_CONNECTEX};

use winapi::shared::ws2def::*;
use winapi::shared::inaddr::*;
use winapi::shared::guiddef::{GUID};
use winapi::um::ws2tcpip::*;
use winapi::um::synchapi::*;
use winapi::um::ioapiset::{CancelIoEx,GetOverlappedResult};
use winapi::um::errhandlingapi::{GetLastError,SetLastError};
use winapi::um::minwinbase::{OVERLAPPED,LPSECURITY_ATTRIBUTES,SECURITY_ATTRIBUTES,LPOVERLAPPED};
use winapi::um::winbase::{INFINITE,WAIT_OBJECT_0};
use winapi::um::handleapi::{CloseHandle};
use winapi::shared::minwindef::{MAKEWORD,WORD,LOBYTE,HIBYTE,BOOL,DWORD,TRUE,FALSE,LPVOID,LPDWORD};
use winapi::ctypes::{c_int,c_char,c_void};
use winapi::shared::winerror::{ERROR_IO_INCOMPLETE,ERROR_IO_PENDING,ERROR_NOT_FOUND};

use super::{evtcall_error_class,evtcall_new_error};
use super::consts_windows::{NULL_HANDLE_VALUE};
use super::consts::*;
use std::error::Error;
use crate::logger::*;
use crate::*;
use crate::sockhdltype::{TcpSockType};

evtcall_error_class!{SockHandleError}

const INET_ADDRSTRLEN :usize = 16;


pub struct TcpSockHandle {
	mtype : TcpSockType,
	inconn : i32,
	sock : SOCKET,
	connov :OVERLAPPED,
	inrd :i32,
	rdov :OVERLAPPED,
	inwr :i32,
	wrov :OVERLAPPED,
	inacc :i32,
	accov :OVERLAPPED,
	acceptfunc : LPFN_ACCEPTEX,
	connexfunc :LPFN_CONNECTEX,
	localaddr : String,
	localport : u32,
	peeraddr :String,
	peerport : u32,
	accsock :SOCKET,
	ooaccrd :DWORD,
	accrdbuf : Vec<u8>,
	oordbuf :Vec<u8>,
	oordlen : DWORD,
	rdptr :*mut i8,
	rdlen : u32,
	wrptr :*mut i8,
	wrlen : u32,
	iscloseerr : bool,
}

impl Drop for TcpSockHandle {
	fn drop(&mut self) {
		self.close();
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

macro_rules! get_errno_direct {
	() => {{
		let retv :u32 ;
		unsafe {
			retv = GetLastError() ;
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

macro_rules! get_wsa_errno_direct {
	() => {{
		let retv :c_int;
		unsafe {
			retv = WSAGetLastError();
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
		evtcall_log_trace!("close {} 0x{:x}",$name,$hdval as u64);
		if $hdval != NULL_HANDLE_VALUE {
			unsafe {
				_bret = CloseHandle($hdval);
			}
			if _bret == FALSE {
				_errval = get_errno!();
				evtcall_log_warn!("CloseHandle {} error {}",$name,_errval);
			}
		}
		$hdval = NULL_HANDLE_VALUE;
	};
}

macro_rules! close_socket_safe {
	($sockval :expr , $name :expr) => {
		let _errval :i32;
		let _iret :c_int;
		evtcall_log_trace!("close {} sock 0x{:x}",$name,$sockval as u64);
		if $sockval != INVALID_SOCKET  {
			unsafe {
				_iret = closesocket($sockval);
			}

			if _iret == SOCKET_ERROR {
				_errval = get_errno!();
				evtcall_log_warn!("close 0x{:x} {} error {}",$sockval,$name,_errval);
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
	($hd :expr,$name :expr) => {
		let _errval :i32;
		let _pattr :LPSECURITY_ATTRIBUTES = std::ptr::null_mut::<SECURITY_ATTRIBUTES>() as LPSECURITY_ATTRIBUTES;
		let _pstr :LPCWSTR = std::ptr::null() as LPCWSTR;
		$hd = unsafe {CreateEventW(_pattr,TRUE,FALSE,_pstr)};
		if $hd == NULL_HANDLE_VALUE {
			_errval = get_errno!();
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
			if _bret == FALSE {
				unsafe {
					_errval = GetLastError();
				}
				if _errval != ERROR_NOT_FOUND {
					evtcall_log_warn!("CancelIoEx {} error {}",$name,_errval);	
				}
				
			}
		}
		$val = 0;
	};
}

impl TcpSockHandle {
	pub fn close(&mut self) {

		self.mtype = TcpSockType::SockNoneType;
		cancel_io_safe!(self.inconn,self.sock,self.connov,"connov");
		close_handle_safe!(self.connov.hEvent,"connov handle");

		cancel_io_safe!(self.inrd,self.sock,self.rdov,"rdov");
		close_handle_safe!(self.rdov.hEvent,"rdov handle");		

		cancel_io_safe!(self.inwr,self.sock,self.wrov,"wrov");
		close_handle_safe!(self.wrov.hEvent,"wrov handle");

		cancel_io_safe!(self.inacc,self.sock,self.accov,"accov");
		close_handle_safe!(self.accov.hEvent,"accov handle");

		close_socket_safe!(self.accsock,"accsock handle");
		close_socket_safe!(self.sock,"sock handle");

		self.acceptfunc = None;
		self.connexfunc = None;

		self.localaddr = "".to_string();
		self.localport = 0;

		self.peeraddr = "".to_string();
		self.peerport = 0;

		self.ooaccrd = 0;
		self.accrdbuf = Vec::new();

		self.oordbuf = Vec::new();
		self.oordlen = 0;

		self.rdptr = std::ptr::null_mut::<i8>();
		self.rdlen = 0;
		self.wrptr = std::ptr::null_mut::<i8>();
		self.wrlen = 0;

		self.iscloseerr = false;
		return;
	}

	fn _format_sockaddr_in(&self,ipaddr :&str,port :u32) -> SOCKADDR_IN {
		let mut name :SOCKADDR_IN = unsafe{ std::mem::zeroed()};
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
		return name;
	}

	fn _default_new(socktype :TcpSockType) -> Self {
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
			connexfunc : None,
			localaddr : format!(""),
			peeraddr : format!(""),
			peerport : 0,
			localport : 0,
			ooaccrd : 0,
			accrdbuf : Vec::new(),
			oordbuf : Vec::new(),
			oordlen : 0,
			rdptr : std::ptr::null_mut::<i8>(),
			rdlen : 0,
			wrptr : std::ptr::null_mut::<i8>(),
			wrlen : 0,
			iscloseerr : false,
		}
	}

	fn _bind_addr(&mut self,ipaddr :&str, port :u32) -> Result<(),Box<dyn Error>> {
		let name :SOCKADDR_IN;
		let ret :i32;
		let iret :c_int;
		let namelen :c_int;

		name = self._format_sockaddr_in(ipaddr,port);
		
		namelen = std::mem::size_of::<SOCKADDR_IN>() as c_int;
		evtcall_debug_buffer_trace!(&name as *const SOCKADDR_IN,namelen,"name set");
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
		evtcall_debug_buffer_trace!((guid as *const GUID)as *const u8,std::mem::size_of::<GUID>(),"acceptfunc get value {:p}",self.acceptfunc.as_ref().unwrap());
		/*#[cfg(target_pointer_width  = "64")]
		{
			let np :*const u64 = unsafe { std::mem::transmute(self.acceptfunc.as_ref().unwrap()) };
			let cp :u64 = unsafe{*np};
			let fs :unsafe extern "system" fn(
				dwError: DWORD,
				cbTransferred: DWORD,
				lpOverlapped: LPWSAOVERLAPPED,
				dwFlags: DWORD,
				) -> () = unsafe {std::mem::transmute(cp)};
			let ps :*const u8 = unsafe {std::mem::transmute(fs)};
			evtcall_log_trace!("ps {:p}",ps);
			evtcall_debug_buffer_trace!(ps ,0x20,"accept func dump");
		}

		#[cfg(target_pointer_width  = "32")]
		{
			let np :*const u32 = unsafe { std::mem::transmute(self.acceptfunc.as_ref().unwrap()) };
			let cp :u32 = unsafe{*np};
			let fs :unsafe extern "system" fn(
				dwError: DWORD,
				cbTransferred: DWORD,
				lpOverlapped: LPWSAOVERLAPPED,
				dwFlags: DWORD,
				) -> () = unsafe {std::mem::transmute(cp)};
			let ps :*const u8 = unsafe {std::mem::transmute(fs)};
			evtcall_log_trace!("ps {:p}",ps);
			evtcall_debug_buffer_trace!(ps ,0x20,"accept func dump");			
		}*/

		//evtcall_log_trace!("acceptfunc {:p}",self.acceptfunc.as_ref().unwrap());
		Ok(())
	}

	fn _inner_accept(&mut self) -> Result<i32,Box<dyn Error>> {
		let ret :i32;
		let bret :BOOL;
		let mut dret : DWORD = 0;
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
		evtcall_log_trace!("accsock 0x{:x} self {:p}",self.accsock,self);
		self.ooaccrd = 0;
		self.inacc = 0;
		self.accrdbuf = Vec::new();
		for _ in 0..1024 {
			self.accrdbuf.push(0);
		}
		assert!(self.acceptfunc.is_some());
		//evtcall_debug_buffer_trace!(self.accrdbuf.as_ptr(),1024,"accrdbuf");

		unsafe {
			let _dretptr = (&mut dret) as LPDWORD;
			let _outbuf = (self.accrdbuf.as_mut_ptr() as *mut u8) as LPVOID;
			let _localaddrlen = (std::mem::size_of::<SOCKADDR_IN>() + 16) as DWORD;
			//let _outlen = 1024 - (_localaddrlen * 2);
			let _outlen =  0;
			let _ovptr = &mut self.accov as LPOVERLAPPED;
			evtcall_log_trace!("accept sock  0x{:x} accsock 0x{:x} _outbuf {:p} _localaddrlen 0x{:x} _outlen 0x{:x} dret {}",self.sock,self.accsock,_outbuf,_localaddrlen,_outlen,dret);

			#[cfg(target_pointer_width = "64")]
			{
				let np :*const u64 = std::mem::transmute(self.acceptfunc.as_ref().unwrap()) ;
				let cp :u64 = *np;
				let _callfn :unsafe extern "system" fn(
					sListenSocket: SOCKET,
					sAcceptSocket: SOCKET,
					lpOutputBuffer: PVOID,
					dwReceiveDataLength: DWORD,
					dwLocalAddressLength: DWORD,
					dwRemoteAddressLength: DWORD,
					lpdwBytesReceived: LPDWORD,
					lpOverlapped: LPOVERLAPPED,
					) -> BOOL = std::mem::transmute(cp);
				//bret = _callfn(self.sock,self.accsock,_outbuf,0,_localaddrlen,_localaddrlen,_dretptr,_ovptr);
				//bret = FALSE;
				//set_errno!(WSA_IO_PENDING);
			}

			#[cfg(target_pointer_width = "32")]
			{
				let np :*const u32 =  std::mem::transmute(self.acceptfunc.as_ref().unwrap());
				let cp :u32 = *np;
				let callfn :unsafe extern "system" fn(
					sListenSocket: SOCKET,
					sAcceptSocket: SOCKET,
					lpOutputBuffer: PVOID,
					dwReceiveDataLength: DWORD,
					dwLocalAddressLength: DWORD,
					dwRemoteAddressLength: DWORD,
					lpdwBytesReceived: LPDWORD,
					lpOverlapped: LPOVERLAPPED,
					) -> BOOL = std::mem::transmute(cp);
				//bret = callfn(self.sock,self.accsock,_outbuf,0,_localaddrlen,_localaddrlen,_dretptr,_ovptr);
			}

			bret = self.acceptfunc.as_ref().unwrap()(self.sock,self.accsock,_outbuf,_outlen,_localaddrlen,_localaddrlen,_dretptr,_ovptr);
		}

		if bret == FALSE {
			ret = get_wsa_errno!();
			evtcall_log_trace!("self.sock 0x{:x} self.accsock 0x{:x} self {:p}",self.sock,self.accsock,self);
			if ret != -WSA_IO_PENDING {
				evtcall_new_error!{SockHandleError,"call acceptfunc error {}",ret}
			}
			self.inacc = 1;
		} else {
			self.inacc = 0;
			self.ooaccrd = dret;
		}

		Ok(self.inacc)
	}

	pub fn bind_server(ipaddr :&str,port :u32,backlog : i32) -> Result<Self,Box<dyn Error>> {
		if ipaddr.len() == 0 || port == 0 {
			evtcall_new_error!{SockHandleError,"not valid ipaddr [{}] or port [{}]",ipaddr,port}
		}
		let mut retv :Self = Self::_default_new(TcpSockType::SockServerType);
		let ret :i32;
		let mut iret :c_int;
		let opt :c_int;
		let mut block :u_long;
		let accguid:GUID = WSAID_ACCEPTEX;

		retv.localaddr = format!("{}",ipaddr);
		retv.localport = port;
		unsafe {
			retv.sock = socket(AF_INET,SOCK_STREAM,0);
		}

		if retv.sock == INVALID_SOCKET {
			ret = get_errno!();
			evtcall_new_error!{SockHandleError,"cannot socket error {}",ret}
		}
		evtcall_log_trace!("sock 0x{:x}",retv.sock);

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

		create_event_safe!(retv.accov.hEvent,"connov handle");

		evtcall_log_trace!("sock 0x{:x} backlog {}",retv.sock,backlog);
		unsafe {
			iret = listen(retv.sock,backlog);
		}
		if iret == SOCKET_ERROR {
			ret = get_wsa_errno!();
			evtcall_new_error!{SockHandleError,"cannot listen error {}",ret}

		}

		retv._get_accept_func(&accguid)?;
		let _ = retv._inner_accept()?;

		Ok(retv)
	}

	pub fn get_accept_handle(&self) -> u64 {
		let mut retv :u64 = INVALID_EVENT_HANDLE;
		if self.inacc > 0 {
			retv = self.accov.hEvent as u64;
		}
		return retv;
	}

	pub fn get_connect_handle(&self) -> u64 {
		let mut retv :u64 = INVALID_EVENT_HANDLE;

		if self.inconn > 0 {
			retv = self.connov.hEvent as u64;
		}
		return retv;
	}

	pub fn get_read_handle(&self) -> u64 {
		let mut retv :u64 = INVALID_EVENT_HANDLE;
		if self.inrd > 0 {
			retv = self.rdov.hEvent as u64;
		}
		return retv;
	}

	pub fn get_write_handle(&self) -> u64 {
		let mut retv :u64 = INVALID_EVENT_HANDLE;
		if self.inwr > 0 {
			retv = self.wrov.hEvent as u64;
		}
		return retv;
	}


	pub fn complete_accept(&mut self) -> Result<i32,Box<dyn Error>> {
		let mut completed :i32 = 0;
		let mut dret :DWORD = 0;
		let ret :u32;

		evtcall_log_trace!("self {:p}",self);
		match self.mtype {
			TcpSockType::SockServerType => {},
			_ => {evtcall_new_error!{SockHandleError,"not valid type for accept"}}
		}

		if self.inacc > 0 {
			let bret :BOOL;
			evtcall_log_trace!("sock 0x{:x}",self.sock);
			evtcall_log_trace!("self.sock 0x{:x} self.accsock 0x{:x} self {:p}",self.sock,self.accsock,self);
			set_errno!(0);
			unsafe {
				let _hd = self.sock as HANDLE;
				let _ovptr = (&mut self.accov) as LPOVERLAPPED;
				let _dptr = (&mut dret) as LPDWORD;
				evtcall_log_trace!("_dptr {:p} dret {}",_dptr,dret);
				bret = GetOverlappedResult(_hd,_ovptr,_dptr,FALSE);
			}

			if bret == FALSE {
				ret = get_errno_direct!();
				evtcall_log_trace!("sock 0x{:x} bret {} ret {}",self.sock,bret,ret);
				if ret == ERROR_IO_INCOMPLETE || ret == ERROR_IO_PENDING {
					/*not completed*/
					return Ok(0);
				}
				evtcall_new_error!{SockHandleError,"complete accept error {}",ret}
			}
			evtcall_log_trace!("bret {} dret {} 0x{:x}",bret,dret,dret);
			//evtcall_debug_buffer_trace!(self.accrdbuf.as_ptr(),self.accrdbuf.len(),"accept read");
			evtcall_log_trace!("self.sock 0x{:x} self.accsock 0x{:x}",self.sock,self.accsock);
			//self.ooaccrd = dret;
			if self.ooaccrd > 0 {
				evtcall_debug_buffer_trace!(self.accrdbuf.as_ptr(),self.ooaccrd,"accept read buffer");
			}
			self.inacc = 0;
			completed = 1;
			evtcall_log_trace!(" ");
		}
		Ok(completed)
	}

	fn _get_peer_name(&mut self) -> Result<(),Box<dyn Error>> {
		let mut name :SOCKADDR_IN = unsafe {std::mem::zeroed()};
		let ret :i32;
		let eret :i32;

		unsafe {
			let _nameptr = (&mut name as *mut SOCKADDR_IN) as * mut SOCKADDR;
			let mut _namelen:i32 = std::mem::size_of::<SOCKADDR_IN>() as i32;
			ret = getpeername(self.sock,_nameptr,&mut _namelen);
		}

		if ret != 0 {
			eret = get_wsa_errno!();
			evtcall_new_error!{SockHandleError,"get peer name error {}",eret}
		}

		if name.sin_family != AF_INET as u16 {
			evtcall_new_error!{SockHandleError,"sock not AF_INET {}",name.sin_family}
		}

		let mut svec :Vec<u8> = Vec::new();
		for _ in 0..INET_ADDRSTRLEN {
			svec.push(0);
		}

		unsafe {
			let _nptr = (&name.sin_addr as *const IN_ADDR) as * const c_void;
			let _ipptr = svec.as_ptr() as *mut u8 as PSTR;
			let _iplen = INET_ADDRSTRLEN;
			inet_ntop(AF_INET,_nptr,_ipptr,_iplen);
		}

		let mut cv :Vec<u8> = Vec::new();
		for c in svec.iter() {
			if *c == 0 {
				break;
			}
			cv.push(*c);
		}

		self.peeraddr = String::from_utf8_lossy(&cv).to_string();
		self.peerport = unsafe{ ntohs(name.sin_port)} as u32;

		Ok(())
	}

	fn _inner_make_read_write(&mut self) -> Result<(),Box<dyn Error>> {
		create_event_safe!(self.rdov.hEvent,"rdov event");
		create_event_safe!(self.wrov.hEvent,"wrov event");
		Ok(())
	}

	pub fn accept_socket(&mut self) -> Result<Self,Box<dyn Error>> {
		let mut retv :Self = Self::_default_new(TcpSockType::SockServerConnType);
		let sret :c_int;
		let sv :i32;
		evtcall_log_trace!(" ");
		match self.mtype {
			TcpSockType::SockServerType => {

			},
			_ => {
				evtcall_new_error!{SockHandleError,"not valid type for accept socket"}
			}
		}

		if self.accsock == INVALID_SOCKET || self.inacc > 0 {
			evtcall_new_error!{SockHandleError,"not valid state for accept socket"}
		}
		retv.localaddr = format!("{}",self.localaddr);
		retv.localport = self.localport;
		evtcall_log_trace!(" ");

		/*to make */
		retv.sock = self.accsock;
		self.accsock = INVALID_SOCKET;

		evtcall_log_trace!("retv.sock 0x{:x} self.sock 0x{:x}",retv.sock,self.sock);

		unsafe {
			let _sptr = ((&self.sock) as *const SOCKET) as *const c_char;
			let _slen = std::mem::size_of::<SOCKET>() as i32;
			evtcall_log_trace!("_sptr {:p} _slen {}",_sptr,_slen);
			sret = setsockopt(retv.sock,SOL_SOCKET,SO_UPDATE_ACCEPT_CONTEXT,_sptr,_slen);
		}

		//evtcall_log_trace!(" ");

		if sret != 0 {
			sv = get_wsa_errno_direct!();
			evtcall_log_trace!("retv.sock 0x{:x} self.sock 0x{:x} get sv {}",retv.sock,self.sock,sv);
			evtcall_new_error!{SockHandleError,"get [{}:{}] SO_UPDATE_ACCEPT_CONTEXT error sret {} {}",retv.localaddr,retv.localport,sret,sv}
		}

		if self.ooaccrd > 0 {
			retv.oordbuf = self.accrdbuf.clone();
			retv.oordlen = self.ooaccrd;			
		}

		retv._get_peer_name()?;
		retv._inner_make_read_write()?;

		let _ = self._inner_accept()?;

		Ok(retv)
	}

	fn _get_connect_func(&mut self, guid :&GUID) -> Result<(),Box<dyn Error>> {
		let mut iret :c_int;
		let mut dret :DWORD = 0;
		unsafe {
			let _funcptr :LPVOID = (&mut self.connexfunc as *mut LPFN_CONNECTEX ) as LPVOID;
			let _funcsize:DWORD = std::mem::size_of::<LPFN_CONNECTEX>() as DWORD;
			let _guidptr :LPVOID = (guid as *const GUID) as LPVOID;
			let _guidsize :DWORD = std::mem::size_of::<GUID>() as DWORD;
			let _retptr :LPDWORD = &mut dret;
			let _ptrov :LPWSAOVERLAPPED = std::ptr::null::<WSAOVERLAPPED>() as LPWSAOVERLAPPED;
			let _fncall :LPWSAOVERLAPPED_COMPLETION_ROUTINE = None;
			iret = WSAIoctl(self.sock,SIO_GET_EXTENSION_FUNCTION_POINTER,_guidptr,_guidsize,_funcptr,_funcsize,_retptr,_ptrov,_fncall);
		}

		if iret != 0 {
			iret = get_wsa_errno!();
			evtcall_new_error!{SockHandleError,"cannot get WSAIoctl connexfunc error {}",iret}
		}

		if self.connexfunc.is_none() {
			evtcall_new_error!{SockHandleError,"connexfunc none"}
		}
		Ok(())
	}

	fn _call_connect_func(&mut self,ipaddr :&str,port :u32) -> Result<i32,Box<dyn Error>> {
		let mut name :SOCKADDR_IN ;
		let mut dret : DWORD =0;
		let wsaerr :c_int;
		let mut completed :i32 = 1;
		let bret :BOOL;
		let mut errval :c_int = 0;
		let iret :c_int;

		name = self._format_sockaddr_in(ipaddr,port);

		unsafe {
			let _eptr  = (&mut errval as *mut c_int) as *mut i8;
			let _elen = std::mem::size_of::<c_int>() as i32;
			iret = setsockopt(self.sock,SOL_SOCKET,SO_ERROR,_eptr,_elen);			
		}

		if iret != 0 {
			evtcall_new_error!{SockHandleError,"cannot set setsockopt error 0 {}",iret}
		}


		unsafe {
			let _namelen = std::mem::size_of::<SOCKADDR_IN>() as i32;
			let _nameptr = (&mut name as * mut SOCKADDR_IN) as *mut SOCKADDR;
			let _dretptr = &mut dret;
			let _ovptr = &mut self.connov;
			let _sndbuf = std::ptr::null_mut() as *mut c_void;
			bret = self.connexfunc.as_ref().unwrap()(self.sock,_nameptr,_namelen,_sndbuf,0,_dretptr,_ovptr);
		}

		if bret == FALSE {
			wsaerr = get_wsa_errno_direct!();
			evtcall_log_trace!("wsaerr {}",wsaerr);
			if wsaerr != (ERROR_IO_PENDING as i32){
				evtcall_new_error!{SockHandleError,"connexfunc call error {}",wsaerr}
			} 
			completed = 0;
		}
		evtcall_log_trace!("completed {}",completed);
		Ok(completed)
	}

	fn _get_self_name(&mut self) -> Result<(),Box<dyn Error>> {
		let mut name :SOCKADDR_IN = unsafe {std::mem::zeroed()};
		let ret :i32;
		let eret :i32;

		unsafe {
			let _nameptr = (&mut name as *mut SOCKADDR_IN) as * mut SOCKADDR;
			let mut _namelen:i32 = std::mem::size_of::<SOCKADDR_IN>() as i32;
			ret = getsockname(self.sock,_nameptr,&mut _namelen);
		}

		if ret != 0 {
			eret = get_wsa_errno!();
			evtcall_new_error!{SockHandleError,"get sock name error {}",eret}
		}

		if name.sin_family != AF_INET as u16 {
			evtcall_new_error!{SockHandleError,"sock not AF_INET {}",name.sin_family}
		}

		let mut svec :Vec<u8> = Vec::new();
		for _ in 0..INET_ADDRSTRLEN {
			svec.push(0);
		}
		unsafe {
			let _nptr = (&name.sin_addr as *const IN_ADDR) as * const c_void;
			let _ipptr = svec.as_ptr() as *mut u8 as PSTR;
			let _iplen = INET_ADDRSTRLEN;
			inet_ntop(AF_INET,_nptr,_ipptr,_iplen);
		}

		let mut cv :Vec<u8> = Vec::new();
		for c in svec.iter() {
			if *c == 0 {
				break;
			}
			cv.push(*c);
		}

		self.localaddr = String::from_utf8_lossy(&cv).to_string();
		self.localport = unsafe{ ntohs(name.sin_port)} as u32;

		Ok(())

	}


	pub fn connect_client(ipaddr :&str,port :u32,localip :&str, localport :u32, connected :bool) -> Result<Self,Box<dyn Error>> {
		let mut retv :Self = Self::_default_new(TcpSockType::SockClientType);
		let mut eret :u32;
		let mut block :u_long;
		let mut iret :c_int;
		let guid :GUID = WSAID_CONNECTEX;
		let mut bret :BOOL;
		let mut dret :DWORD;

		retv.peeraddr = format!("{}",ipaddr);
		retv.peerport = port;
		unsafe {
			retv.sock = socket(AF_INET,SOCK_STREAM,0);
		}


		evtcall_log_trace!("sock 0x{:x}",retv.sock);
		if retv.sock == INVALID_SOCKET {
			iret = get_wsa_errno!();
			evtcall_new_error!{SockHandleError,"socket client error {}",iret}
		}

		block = 1;
		unsafe {
			let _bptr = (&mut block) as *mut u_long;
			iret = ioctlsocket(retv.sock,FIONBIO,_bptr);
		}

		if iret == SOCKET_ERROR {
			iret = get_wsa_errno!();
			evtcall_new_error!{SockHandleError,"ioctlsocket FIONBIO error {}",iret}
		}

		retv._bind_addr(localip,localport)?;


		create_event_safe!(retv.connov.hEvent,"connov event");

		retv._get_connect_func(&guid)?;
		let completed = retv._call_connect_func(ipaddr,port)?;
		evtcall_log_trace!("_call_connect_func completed {}",completed);
		if completed == 0 {
			retv.inconn = 1;
		} else {
			retv.inconn = 0;
			retv._get_self_name()?;
			retv._inner_make_read_write()?;
		}

		evtcall_log_trace!(" ");

		if retv.inconn > 0 && connected {
			loop {
				unsafe {
					dret = WaitForSingleObject(retv.connov.hEvent,INFINITE);
				}
				if dret == WAIT_OBJECT_0 {
					unsafe {
						let _ovptr = &mut retv.connov;
						let _dretptr = &mut dret;
						bret = GetOverlappedResult(retv.sock as HANDLE,_ovptr,_dretptr,FALSE);
					}
					if bret == TRUE {
						break;
					}
					eret = get_errno_direct!();
					if eret == ERROR_IO_INCOMPLETE || eret == ERROR_IO_PENDING {
						continue;
					}
					evtcall_new_error!{SockHandleError,"GetOverlappedResult error {}",eret}
				} else {
					eret = get_errno_direct!();
					evtcall_new_error!{SockHandleError,"WaitForSingleObject error {} {}",dret,eret}
				}
			}

			/*we already connected*/
			retv.inconn = 0;
			retv._get_self_name()?;
			retv._inner_make_read_write()?;
		}

		evtcall_log_trace!("connect [{}:{}] inconn {} self {:p}",ipaddr,port,retv.inconn,&retv);
		Ok(retv)
	}

	pub fn is_close_error(&self) -> bool {
		return self.iscloseerr;
	}

	fn _inner_read(&mut self) ->Result<(),Box<dyn Error>> {
		let mut iret :c_int;
		let mut rdbuf :WSABUF = unsafe {std::mem::zeroed()};
		let mut flags : DWORD;
		let eret :i32;
		let mut dret  :DWORD=0;
		loop {
			rdbuf.len = self.rdlen;
			rdbuf.buf = self.rdptr;
			flags = MSG_PARTIAL as DWORD;
			set_errno!(0);
			unsafe {
				let _rdptr = &mut rdbuf;
				let _flagptr = &mut flags;
				let _dretptr = &mut dret;
				let _ovptr = &mut self.rdov;
				//let _funcptr = None;
				iret = WSARecv(self.sock,_rdptr,1,_dretptr,_flagptr,_ovptr,None);
			}

			if iret == 0 {
				evtcall_log_trace!("dret {}",dret);
				if dret == 0 {
					self.iscloseerr = true;
					evtcall_new_error!{SockHandleError,"closed local[{}:{}] peer[{}:{}]",self.localaddr,self.localport,self.peeraddr,self.peerport}
				}

				self.rdlen -= dret;
				self.rdptr = unsafe{ self.rdptr.offset(dret as isize)};

				if self.rdlen == 0 {
					self.inrd = 0;
					self.rdlen = 0;
					self.rdptr = std::ptr::null_mut::<i8>();
					return Ok(());
				}
				continue;
			}

			eret = get_wsa_errno_direct!();
			if eret == WSA_IO_PENDING {
				return Ok(());
			}
			evtcall_log_error!("read  peer[{}:{}] => local[{}:{}] error [{}] iret {}",self.peeraddr,self.peerport,self.localaddr,self.localport,eret,iret);
			evtcall_new_error!{SockHandleError,"read  peer[{}:{}] => local[{}:{}] error [{}]",self.peeraddr,self.peerport,self.localaddr,self.localport,eret}
		}
	}

	fn _inner_write(&mut self) ->Result<(),Box<dyn Error>> {
		let mut iret :c_int;
		let mut wrbuf :WSABUF = unsafe {std::mem::zeroed()};
		let mut flags : DWORD;
		let eret :i32;
		let mut dret  :DWORD=0;
		loop {
			wrbuf.len = self.wrlen;
			wrbuf.buf = self.wrptr;
			flags = MSG_PARTIAL as DWORD;
			unsafe {
				let _wrptr = &mut wrbuf;
				let _flagptr = &mut flags;
				let _dretptr = &mut dret;
				let _ovptr = &mut self.wrov;
				let _funcptr = None;
				iret = WSASend(self.sock,_wrptr,1,_dretptr,flags,_ovptr,_funcptr);
			}

			if iret == 0 {

				self.wrlen -= dret;
				self.wrptr = unsafe{ self.wrptr.offset(dret as isize)};

				if self.wrlen == 0 {
					self.inwr = 0;
					self.wrlen = 0;
					self.wrptr = std::ptr::null_mut::<i8>();
					return Ok(());
				}
				continue;
			}

			eret = get_wsa_errno_direct!();
			if eret == WSA_IO_PENDING {
				return Ok(());
			}
			evtcall_new_error!{SockHandleError,"WSASend local[{}:{}] peer[{}:{}] error [{}]",self.localaddr,self.localport,self.peeraddr,self.peerport,eret}
		}
	}

	pub fn complete_connect(&mut self) -> Result<i32,Box<dyn Error>> {
		let mut completed :i32=1;
		let bret :BOOL;
		let mut dret : DWORD = 0;
		let retu :u32;
		let wsaerru :c_int;
		let mut errval :c_int = 0;
		if self.inconn > 0 {
			unsafe {
				let _hd = self.sock as HANDLE;
				let _ovptr = &mut self.connov;
				let _dretptr = &mut dret;
				SetLastError(0);
				bret = GetOverlappedResult(_hd,_ovptr,_dretptr,FALSE);
			}
			if bret == FALSE {
				retu = get_errno_direct!();
				wsaerru = get_wsa_errno_direct!();
				evtcall_log_trace!("retu {} wsaerru {}",retu,wsaerru);

				unsafe {
					let _eptr = (&mut errval as *mut c_int) as *mut i8;
					let mut _elen :i32 = std::mem::size_of::<c_int>() as i32;
					getsockopt(self.sock,SOL_SOCKET,SO_ERROR,_eptr,&mut _elen);
				}

				evtcall_log_trace!("errval {}",errval);

				if retu == ERROR_IO_PENDING {
					return Ok(0);
				}
				evtcall_new_error!{SockHandleError,"get connov result error {}",retu}
			}
			self.inconn = 0;
			self._get_self_name()?;
			self._inner_make_read_write()?;
		}

		if self.inconn > 0 {
			completed = 0;
		}
		Ok(completed)
	}


	pub fn complete_read(&mut self) -> Result<i32,Box<dyn Error>> {
		let bret :BOOL;
		let eret :u32;
		let mut dret :DWORD = 0;
		if self.inrd > 0 {
			unsafe {
				let _hd = self.sock as HANDLE;
				let _ovptr = (&mut self.rdov) as LPOVERLAPPED;
				let _dptr = (&mut dret) as LPDWORD;
				bret = GetOverlappedResult(_hd,_ovptr,_dptr,FALSE);
			}

			if bret == FALSE {
				eret = get_errno_direct!();
				evtcall_new_error!{SockHandleError,"get read ov error {}",eret}
			}

			self.rdptr = unsafe{self.rdptr.offset(dret as isize)};
			self.rdlen -= dret;
			if self.rdlen == 0 {
				self.inrd = 0;
				self.rdptr = std::ptr::null_mut::<i8>();
				self.rdlen = 0;
			} else {
				self._inner_read()?;
			}
		}

		let mut completed :i32 = 1;
		if self.inrd > 0 {
			completed = 0;
		}
		Ok(completed)
	}

	pub fn complete_write(&mut self) -> Result<i32,Box<dyn Error>> {
		let mut dret :DWORD = 0;
		let bret :BOOL;
		let eret :u32;
		if self.inwr > 0 {
			unsafe {
				let _hd = self.sock as HANDLE;
				let _ovptr = (&mut self.wrov) as LPOVERLAPPED;
				let _dptr = (&mut dret) as LPDWORD;
				bret = GetOverlappedResult(_hd,_ovptr,_dptr,FALSE);
			}

			if bret == FALSE {
				eret = get_errno_direct!();
				evtcall_new_error!{SockHandleError,"get read ov error {}",eret}
			}

			self.wrptr = unsafe{self.wrptr.offset(dret as isize)};
			self.wrlen -= dret;
			if self.wrlen == 0 {
				self.inwr = 0;
				self.wrptr = std::ptr::null_mut::<i8>();
				self.wrlen = 0;
			} else {
				self._inner_write()?;
			}
		}

		let mut completed :i32 = 1;
		if self.inwr > 0 {
			completed = 0;
		}
		Ok(completed)
	}

	pub fn read(&mut self,rbuf :*mut u8, rlen :u32) -> Result<i32,Box<dyn Error>> {
		if self.inacc > 0 || self.inconn >0 || self.inrd > 0 {
			evtcall_log_error!("not valid state inacc {} inconn {} inrd {}",self.inacc,self.inconn,self.inrd);
			evtcall_new_error!{SockHandleError,"not valid state"}
		}

		if self.sock == INVALID_SOCKET {
			evtcall_new_error!{SockHandleError,"closed socket"}
		}

		assert!(self.rdptr == std::ptr::null_mut::<i8>());
		assert!(self.rdlen == 0);

		self.rdptr = rbuf as *mut i8;
		self.rdlen = rlen;
		self.inrd = 1;

		self._inner_read()?;
		let mut completed :i32 = 1;

		if self.inrd > 0 {
			completed = 0;
		}

		Ok(completed)
	}

	pub fn write(&mut self,wbuf :*mut u8, wlen :u32) -> Result<i32,Box<dyn Error>> {
		if self.inacc > 0 || self.inconn >0 || self.inwr > 0 {
			evtcall_new_error!{SockHandleError,"not valid state"}
		}

		if self.sock == INVALID_SOCKET {
			evtcall_new_error!{SockHandleError,"closed socket"}
		}

		assert!(self.wrptr == std::ptr::null_mut::<i8>());
		assert!(self.wrlen == 0);

		self.wrptr = wbuf as *mut i8;
		self.wrlen = wlen;
		self.inwr = 1;

		self._inner_write()?;
		let mut completed :i32 = 1;

		if self.inwr > 0 {
			completed = 0;
		}

		Ok(completed)
	}

	pub fn is_accept_mode(&self) -> bool {
		let mut retv :bool = false;
		if self.inacc > 0 {
			retv = true;
		}
		return retv;
	}

	pub fn is_connect_mode(&self) -> bool {
		let mut retv :bool = false;
		if self.inconn > 0 {
			retv = true;
		}
		return retv;
	}

	pub fn is_read_mode(&self) -> bool {
		let mut retv :bool = false;
		if self.inrd > 0 {
			retv = true;
		}
		return retv;
	}

	pub fn is_write_mode(&self) -> bool {
		let mut retv :bool = false;
		if self.inwr > 0 {
			retv = true;
		}
		return retv;
	}

	pub fn get_self_format(&self) -> String {
		evtcall_log_trace!("self {:p} sock 0x{:x} accsock 0x{:x}",self,self.sock,self.accsock);
		return format!("{}:{}",self.localaddr,self.localport);
	}

	pub fn get_peer_format(&self) -> String {
		return format!("{}:{}",self.peeraddr,self.peerport);
	}


}

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

pub fn fini_socket()  {
	unsafe {
		WSACleanup();
	}
}

