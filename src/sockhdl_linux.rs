

use crate::sockhdl::{TcpSockType};
use std::error::Error;
use crate::logger::*;
use crate::*;
use crate::consts::*;

evtcall_error_class!{SockHandleError}
const DEFAULT_SOCKET :i32 = -2;

pub struct TcpSockHandle {
	sock :i32,
	accsock : i32,
	acc_addr : libc::sockaddr_in,
	mtype :TcpSockType,
	inacc :i32,
	inconn :i32,
	inrd :i32,
	inwr :i32,
	rdptr :*mut u8,
	rdlen :u32,
	wrptr :*mut u8,
	wrlen :u32,
	peeraddr :String,
	peerport :u32,
	localaddr :String,
	localport : u32,
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

macro_rules! close_sock_safe {
	($sock :expr,$name :expr) => {
		if $sock >= 0 {
			let mut _reti :libc::c_int;
			unsafe {
				_reti = libc::close($sock);
			}
			if _reti < 0 {
				_reti = get_errno!();
				evtcall_log_error!("close {} error {}",$name,_reti);
			}
		}
		$sock = DEFAULT_SOCKET;
	};
}

impl Drop for TcpSockHandle {
	fn drop(&mut self) {
		self.close();
	}
}


impl TcpSockHandle {

	fn _default_new(socktype :TcpSockType) -> Self {
		Self {
			sock : DEFAULT_SOCKET,
			accsock : DEFAULT_SOCKET,
			acc_addr : unsafe{std::mem::zeroed()},
			mtype : socktype,
			inacc : 0,
			inconn : 0,
			inrd : 0,
			inwr : 0,
			rdptr : std::ptr::null_mut::<u8>(),
			rdlen : 0,
			wrptr : std::ptr::null_mut::<u8>(),
			wrlen : 0,
			peeraddr : "".to_string(),
			peerport : 0,
			localaddr : "".to_string(),
			localport : 0,
		}
	}

	fn _format_sockaddr_in(&self,ipaddr :&str,port :u32) -> Result<libc::sockaddr_in,Box<dyn Error>> {
		let mut retv :libc::sockaddr_in = unsafe {std::mem::zeroed()};
		let ipv4 :std::net::Ipv4Addr = ipaddr.parse()?;
		let octs :[u8; 4] = ipv4.octets();
		let mut cv :u32 = 0;
		let mut idx :usize=0;
		while idx < octs.len() {
			cv |= (octs[idx] as u32) << (8 * idx);
			idx += 1;
		}
		retv.sin_family = libc::AF_INET as u16;
		retv.sin_port = (port as u16).to_be();
		retv.sin_addr = libc::in_addr { s_addr: cv };
		return Ok(retv);
	}

	fn _accept_inner(&mut self) -> Result<(),Box<dyn Error>> {
		let mut reti :i32;
		if self.inacc > 0 || self.accsock >= 0 {
			evtcall_new_error!{SockHandleError,"already in accept"}
		}

		unsafe {
			let mut _slen :u32= std::mem::size_of::<libc::sockaddr_in>() as u32;
			let _nameptr = (&mut self.acc_addr as *mut libc::sockaddr_in) as *mut libc::sockaddr;
			reti = libc::accept(self.sock,_nameptr,&mut _slen);
		}
		if reti < 0 {
			reti = get_errno!();
			if reti != -libc::EAGAIN || reti != -libc::EWOULDBLOCK {
				evtcall_new_error!{SockHandleError,"accept error {}",reti}
			}
			self.inacc = 1;
		} else {
			self.accsock = reti;
			self.inacc = 0;
		}
		Ok(())
	}

	fn _set_nonblock(&self,sock :i32) -> Result<(),Box<dyn Error>> {
		let flags :i32;
		let mut reti :libc::c_int;
		unsafe {
			flags = libc::fcntl(sock,libc::F_GETFL);
			reti = libc::fcntl(sock,libc::F_SETFL,flags | libc::O_NONBLOCK);
		}
		if reti < 0 {
			reti = get_errno!();
			evtcall_new_error!{SockHandleError,"cannot set O_NONBLOCK error {}",reti}
		}
		Ok(())
	}

	fn _bind_addr(&self,ipaddr :&str, port :u32) -> Result<(),Box<dyn Error>> {
		let name :libc::sockaddr_in;
		let mut reti :i32;
		name = self._format_sockaddr_in(ipaddr,port)?;

		unsafe {
			let _nameptr =(&name as *const libc::sockaddr_in) as *const libc::sockaddr;
			let _namelen = std::mem::size_of::<libc::sockaddr_in>() as u32;
			reti = libc::bind(self.sock,_nameptr,_namelen);
		}

		if reti < 0 {
			reti = get_errno!();
			evtcall_new_error!{SockHandleError,"cannot bind [{}:{}] error {}",ipaddr,port,reti}
		}
		Ok(())
	}

	pub fn bind_server(ipaddr :&str,port :u32,backlog : i32) -> Result<Self,Box<dyn Error>> {
		let mut retv : Self = Self::_default_new(TcpSockType::SockServerType);
		let mut reti :i32;
		let opt :libc::c_int;

		unsafe {
			retv.sock = libc::socket(libc::AF_INET,libc::SOCK_STREAM,0);
		}
		if retv.sock < 0 {
			reti = get_errno!();
			evtcall_new_error!{SockHandleError,"can not libc::socket error {}",reti}
		}

		retv._set_nonblock(retv.sock)?;
		retv.localaddr = format!("{}",ipaddr);
		retv.localport = port;

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

		retv._bind_addr(ipaddr,port)?;

		unsafe {
			reti = libc::listen(retv.sock,backlog);
		}

		if reti < 0 {
			reti = get_errno!();
			evtcall_new_error!{SockHandleError,"cannot listen  [{}:{}] backlog {} error {}",ipaddr,port,backlog,reti}	
		}

		retv._accept_inner()?;
		Ok(retv)
	}

	fn _get_sock_name(&mut self) -> Result<(),Box<dyn Error>> {
		let mut name :libc::sockaddr_in = unsafe {std::mem::zeroed()};
		let mut reti :i32;
		unsafe {
			let _nameptr = (&mut name as *mut libc::sockaddr_in) as *mut libc::sockaddr;
			let _namelen = std::mem::size_of::<libc::sockaddr_in>();
			reti = libc::getsockname(self.sock,_nameptr,_namelen);
		}

		if reti < 0 {
			reti = get_errno!();
			evtcall_new_error!{SockHandleError,"getsockname error {}",reti}
		}

		let ipv4 :std::net::Ipv4Addr = std::net::Ipv4Addr::from_bits(name.sin_addr.s_addr);
		retv.localaddr = ipv4.to_string();
		retv.localport = u16::from_be(name.sin_port) as u32;
		Ok(())
	}

	pub fn connect_client(ipaddr :&str,port :u32,localip :&str, localport :u32, connected :bool) -> Result<Self,Box<dyn Error>> {
		let mut retv :Self = Self::_default_new(TcpSockType::SockClientType);
		let mut reti :i32;
		let name : libc::sockaddr_in;
		if ipaddr.len() == 0 || port == 0 || port >= (1 << 16) {
			evtcall_new_error!{SockHandleError,"not valid ipaddr [{}] or port [{}]",ipaddr,port}
		}

		unsafe {
			retv.sock = libc::socket(libc::AF_INET,libc::SOCK_STREAM,0);
		}
		if retv.sock < 0 {
			reti = get_errno!();
			evtcall_new_error!{SockHandleError,"socket error {}",reti}
		}

		retv._set_nonblock(retv.sock)?;
		if localip.len() > 0 && localport != 0 {
			retv.localaddr = format!("{}",localip);
			retv.localport = localport;
			retv._bind_addr(localip,localport)?;
		}

		/*now to set for error*/
		let mut error :libc::c_int = 0;
		unsafe {
			let _eptr = (&mut error as *mut libc::c_int) as *mut libc::c_void;
			let _elen = std::mem::size_of::<libc::c_int>() as u32;
			reti = libc::setsockopt(retv.sock,libc::SOL_SOCKET,libc::SO_ERROR,_eptr,_elen);
		}

		if reti < 0 {
			reti = get_errno!();
			evtcall_new_error!{SockHandleError,"setsockopt SO_ERROR error {}",reti}
		}

		name = retv._format_sockaddr_in(ipaddr,port)?;
		let mut inconn :i32 = 0;

		unsafe {
			let _nameptr = (&name as * const libc::sockaddr_in) as *const libc::sockaddr;
			let _namelen = std::mem::size_of::<libc::sockaddr_in>() as u32;
			reti = libc::connect(retv.sock,_nameptr,_namelen);
		}
		if reti < 0 {
			reti = get_errno!();
			if reti != -libc::EINPGROGRESS {
				evtcall_new_error!{SockHandleError,"connect error {}", reti}	
			}
			inconn = 1;
		}

		if connected && inconn > 0 {
			let mut rdset :libc::fd_set = unsafe {std::mem::zeroed()};
			loop {
				unsafe {
					let _rdptr = &mut rdset as *mut libc::fd_set;
					libc::FD_ZERO(_rdptr);
					libc::FD_SET(retv.sock,_rdptr);
					let _nullptr = std::ptr::null_mut::<libc::fd_set>();
					let _timenull = std::ptr::null_mut::<libc::timeval>();
					reti = libc::select(retv.sock + 1,_rdptr,_nullptr,_nullptr,_timenull);
				}

				if reti < 0 {
					reti = get_errno!();
					evtcall_new_error!{SockHandleError,"select error {}",reti}
				} else if reti == 0 {
					continue;
				} else {
					error = 1;
					unsafe {
						let _eptr = (&mut error as *mut libc::c_int) as *mut libc::c_char;
						let _elen = std::mem::size_of::<libc::c_int>() as u32;
						reti = libc::getsockopt(retv.sock,libc::SOL_SOCKET,libc::SO_ERROR,_eptr,_elen);
					}

					if reti < 0 {
						reti = get_errno!();
						evtcall_new_error!{SockHandleError,"getsockopt SO_ERROR error {}",reti}
					}
					if error != 0 {
						evtcall_new_error!{SockHandleError,"connect [{}:{}] error {}",ipaddr,port,error}
					}
					inconn = 0;
					retv._get_sock_name()?;
					break;
				}
			}
		}
		retv.inconn = inconn;

		Ok(retv)
	}

	pub fn get_accept_handle(&self) -> u64 {
		let mut retv :u64 = INVALID_EVENT_HANDLE;
		if self.inacc > 0 {
			retv = self.sock as u64;
		}
		return retv;
	}

	pub fn get_read_handle(&self) -> u64 {
		let mut retv :u64 = INVALID_EVENT_HANDLE;
		if self.inrd > 0 {
			retv = self.sock as u64;
		}
		return retv;
	}

	pub fn get_write_handle(&self) -> u64 {
		let mut retv :u64 = INVALID_EVENT_HANDLE;
		if self.inwr > 0 {
			retv = self.sock as u64;
		}
		return retv;
	}

	pub fn close(&mut self) {
		close_sock_safe!(self.sock,"sock");
		close_sock_safe!(self.accsock,"accsock");
		self.acc_addr = unsafe{std::mem::zeroed()};
		self.mtype = TcpSockType::SockNoneType;
		self.inacc = 0;
		self.inconn = 0;
		self.inrd = 0;
		self.inwr = 0;
		self.rdptr = std::ptr::null_mut::<u8>();
		self.rdlen = 0;
		self.wrptr = std::ptr::null_mut::<u8>();
		self.wrlen = 0;
		self.localaddr = "".to_string();
		self.localport = 0;
		self.peeraddr = "".to_string();
		self.peerport = 0;
	}
}