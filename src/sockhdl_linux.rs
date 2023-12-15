

use crate::sockhdl::{TcpSockType};
use std::error::Error;
use crate::logger::*;
use crate::*;

evtcall_error_class!{SockHandleError}

pub struct TcpSockHandle {
	sock :i32,
	mtype :TcpSockType,
	inacc :i32,
	inconn :i32,
	inrd :i32,
	inwr :i32,
	rdptr :*mut u8,
	rdlen :u32,
	wrptr :*mut u8,
	wrlen :u32,
}

macro_rules! get_errno {
	() => {{
		let mut _retv :i32;
		unsafe {
			_retv = (*libc::__errno_location())  as i32;
		}
		if _retv > 0 {
			_retv = -_retv;
		} else if _retv == 0 {
			_retv = -1;
		}
		_retv
	}};
}

impl Drop for TcpSockHandle {
	fn drop(&mut self) {
		self.close();
	}
}

#[nolink]
extern "C" {


    fn inet_ntop(af: libc::c_int, src: *libc::c_void, dst: *u8, size: socklen_t) -> c_str;
    fn inet_pton(af: libc::c_int, src: c_str, dst: *libc::c_void) -> libc::c_int;

    fn gai_strerror(ecode: libc::c_int) -> c_str;
    fn getaddrinfo(node: c_str, service: c_str, hints: *addrinfo, res: **addrinfo) -> libc::c_int;
    fn freeaddrinfo(ai: *addrinfo);
}

impl TcpSockHandle {

	fn _default_new(socktype :TcpSockType) -> Self {
		Self {
			sock : -1,
			mtype : socktype,
			inacc : 0,
			inconn : 0,
			inrd : 0,
			inwr : 0,
			rdptr : std::ptr::null_mut::<u8>(),
			rdlen : 0,
			wrptr : std::ptr::null_mut::<u8>(),
			wrlen : 0,			
		}
	}

	fn _format_sockaddr_in(&self,ipaddr :&str,port :u32) -> libc::sockaddr_in {
		let mut name :libc::sockaddr_in = unsafe{ std::mem::zeroed()};
		name.sin_family = libc::AF_INET as u16;
		if ipaddr.len() > 0 {
			unsafe {
				let _pv  = ((&mut name.sin_addr) as * mut libc::in_addr) as *mut libc::c_void;
				let _addr = ipaddr.as_bytes().as_ptr() as * const i8;
				libc::inet_pton(libc::AF_INET,_addr,_pv);
			}			
		} else {
			name.sin_addr = unsafe {std::mem::zeroed()};
		}
		if port != 0 {
			name.sin_port = unsafe {libc::htons(port as u16)};	
		} else {
			name.sin_port = 0;
		}
		return name;
	}

	pub fn bind_server(ipaddr :&str,port :u32,backlog : i32) -> Result<Self,Box<dyn Error>> {
		let mut retv : Self = Self::_default_new(TcpSockType::SockServerType);
		let mut reti :i32;
		let mut opt :libc::c_int;
		let name :libc::sockaddr_in;

		unsafe {
			retv.sock = libc::socket(libc::AF_INET,libc::SOCK_STREAM,0);
		}
		if retv.sock < 0 {
			reti = get_errno!();
			evtcall_new_error!{SockHandleError,"can not libc::socket error {}",reti}
		}

		let flags :i32;
		unsafe {
			flags = libc::fcntl(retv.sock,libc::F_GETFL);
			reti = libc::fcntl(retv.sock,libc::F_SETFL,flags | libc::O_NONBLOCK);
		}
		if reti < 0 {
			reti = get_errno!();
			evtcall_new_error!{SockHandleError,"cannot set O_NONBLOCK error {}",reti}
		}

		opt = 1;
		unsafe {
			let _optptr = (&opt as *const i32) as *const libc::c_void;
			let _optsize = std::mem::size_of::<libc::c_int>() as u32;
			reti = libc::setsockopt(retv.sock,libc::SOL_SOCKET,libc::SO_REUSEADDR,_optptr,_optsize);
		}

		if reti < 0 {
			reti = get_errno!();
			evtcall_new_error!{SockHandleError,"cannot setsockopt SO_REUSEADDR error {}",reti}
		}

		name = retv._format_sockaddr_in(ipaddr,port);

		Ok(retv)
	}

	pub fn close(&mut self) {
		let reti :i32;
		let eret :i32;
		if self.sock >= 0 {
			unsafe {
				reti = libc::close(self.sock);
			}
			if reti < 0 {
				eret = get_errno!();
				evtcall_log_error!("close {} error {}",self.sock,eret);
			}
		}
		self.sock = -1;
		self.mtype = TcpSockType::SockNoneType;
		self.inacc = 0;
		self.inconn = 0;
		self.inrd = 0;
		self.inwr = 0;
		self.rdptr = std::ptr::null_mut::<u8>();
		self.rdlen = 0;
		self.wrptr = std::ptr::null_mut::<u8>();
		self.wrlen = 0;
	}
}