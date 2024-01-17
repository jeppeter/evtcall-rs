

use crate::sockhdltype::{TcpSockType};
use std::error::Error;
use crate::logger::*;
use crate::*;
use crate::consts::*;
use std::sync::Arc;
use std::cell::RefCell;

evtcall_error_class!{SockHandleError}
const DEFAULT_SOCKET :i32 = -2;

struct TcpSockHandleInner {
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

pub struct TcpSockHandle {
	inner :Arc<RefCell<TcpSockHandleInner>>,
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

impl Drop for TcpSockHandleInner {
	fn drop(&mut self) {
		self.close();
	}
}

impl Drop for TcpSockHandle {
	fn drop(&mut self) {
		self.close();
	}
}


impl TcpSockHandleInner {

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

	pub (crate) fn bind_server_after(&mut self,ipaddr :&str, port :u32,backlog :i32) -> Result<(),Box<dyn Error>> {
		let mut reti :i32;
		let opt :libc::c_int;

		unsafe {
			self.sock = libc::socket(libc::AF_INET,libc::SOCK_STREAM,0);
		}
		if self.sock < 0 {
			reti = get_errno!();
			evtcall_new_error!{SockHandleError,"can not libc::socket error {}",reti}
		}

		self._set_nonblock(self.sock)?;
		self.localaddr = format!("{}",ipaddr);
		self.localport = port;

		opt = 1;
		unsafe {
			let _optptr = (&opt as *const i32) as *const libc::c_void;
			let _optsize = std::mem::size_of::<libc::c_int>() as u32;
			reti = libc::setsockopt(self.sock,libc::SOL_SOCKET,libc::SO_REUSEADDR,_optptr,_optsize);
		}

		if reti < 0 {
			reti = get_errno!();
			evtcall_new_error!{SockHandleError,"cannot setsockopt SO_REUSEADDR error {}",reti}
		}

		self._bind_addr(ipaddr,port)?;

		unsafe {
			reti = libc::listen(self.sock,backlog);
		}

		if reti < 0 {
			reti = get_errno!();
			evtcall_new_error!{SockHandleError,"cannot listen  [{}:{}] backlog {} error {}",ipaddr,port,backlog,reti}	
		}

		self._accept_inner()?;
		Ok(())
	}

	pub (crate) fn bind_server(_ipaddr :&str,_port :u32,_backlog : i32) -> Result<Arc<RefCell<Self>>,Box<dyn Error>> {
		let iretv : Self = Self::_default_new(TcpSockType::SockServerType);
		let retv = Arc::new(RefCell::new(iretv));
		Ok(retv)
	}

	pub (crate) fn complete_accept(&mut self) -> Result<i32,Box<dyn Error>> {
		let mut completed :i32= 1;
		match self.mtype {
			TcpSockType::SockServerType => {
				if self.accsock < 0 {
					self._accept_inner()?;
				}
			},
			_ => {}
		}

		if self.inacc > 0 {
			completed = 0;
		}
		Ok(completed)
	}

	fn _get_peer_name(&mut self) -> Result<(),Box<dyn Error>> {
		let mut name :libc::sockaddr_in = unsafe {std::mem::zeroed()};
		let mut reti :i32;
		unsafe {
			let _nameptr = (&mut name as *mut libc::sockaddr_in) as *mut libc::sockaddr;
			let mut _namelen = std::mem::size_of::<libc::sockaddr_in>() as u32;
			reti = libc::getpeername(self.sock,_nameptr,&mut _namelen);
		}
		if reti < 0 {
			reti = get_errno!();
			evtcall_new_error!{SockHandleError,"get peer name error {}",reti}
		}

		let (us,uu) = self._trans_sockaddr_in(&name)?;
		self.peeraddr = format!("{}",us);
		self.peerport = uu;
		Ok(())
	}

	pub (crate) fn accept_socket(&mut self) -> Result<Arc<RefCell<Self>>,Box<dyn Error>> {
		let iretv :Self = Self::_default_new(TcpSockType::SockServerConnType);
		let retv = Arc::new(RefCell::new(iretv));
		match self.mtype {
			TcpSockType::SockServerType => {},
			_ => {evtcall_new_error!{SockHandleError,"not valid type to accept"}}
		}

		if self.accsock < 0 {
			self._accept_inner()?;
			if self.accsock < 0{
				evtcall_new_error!{SockHandleError,"not accepted"}
			}
		}
		assert!(self.inacc == 0);
		retv.borrow_mut().sock = self.accsock;
		self.accsock = DEFAULT_SOCKET;


		retv.borrow_mut().localport = self.localport;
		retv.borrow_mut().localaddr = format!("{}",self.localaddr);

		retv.borrow_mut()._get_peer_name()?;
		self._accept_inner()?;

		Ok(retv)
	}

	fn _trans_sockaddr_in(&self,name :&libc::sockaddr_in) -> Result<(String,u32),Box<dyn Error>> {
		let mut a :Vec<u8> = vec![0,0,0,0];
		let mut idx :usize=0;

		while idx < a.len() {
			a[idx] = ((name.sin_addr.s_addr >> (idx*8)) & 0xff) as u8;
			idx += 1;
		}

		let ipv4 :std::net::Ipv4Addr = std::net::Ipv4Addr::new(a[0],a[1],a[2],a[3]);
		let rets :String = ipv4.to_string();
		let retu :u32 = u16::from_be(name.sin_port) as u32;
		Ok((rets,retu))		
	}

	fn _get_sock_name(&mut self) -> Result<(),Box<dyn Error>> {
		let mut name :libc::sockaddr_in = unsafe {std::mem::zeroed()};
		let mut reti :i32;
		unsafe {
			let _nameptr = (&mut name as *mut libc::sockaddr_in) as *mut libc::sockaddr;
			let mut _namelen = std::mem::size_of::<libc::sockaddr_in>() as u32;
			reti = libc::getsockname(self.sock,_nameptr,&mut _namelen);
		}

		if reti < 0 {
			reti = get_errno!();
			evtcall_new_error!{SockHandleError,"getsockname error {}",reti}
		}
		let (us,uu) = self._trans_sockaddr_in(&name)?;
		self.localaddr = format!("{}",us);
		self.localport = uu;
		Ok(())
	}

	pub (crate) fn connect_client_after(&mut self,ipaddr :&str,port :u32,localip :&str, localport :u32, connected :bool) -> Result<(),Box<dyn Error>> {
		let mut reti :i32;
		let name : libc::sockaddr_in;
		if ipaddr.len() == 0 || port == 0 || port >= (1 << 16) {
			evtcall_new_error!{SockHandleError,"not valid ipaddr [{}] or port [{}]",ipaddr,port}
		}

		unsafe {
			self.sock = libc::socket(libc::AF_INET,libc::SOCK_STREAM,0);
		}
		if self.sock < 0 {
			reti = get_errno!();
			evtcall_new_error!{SockHandleError,"socket error {}",reti}
		}

		self._set_nonblock(self.sock)?;
		if localip.len() > 0 {
			self.localaddr = format!("{}",localip);
			self.localport = localport;
			self._bind_addr(localip,localport)?;
		}

		/*now to set for error*/
		let mut error :libc::c_int = 0;
		unsafe {
			let _eptr = (&mut error as *mut libc::c_int) as *mut libc::c_void;
			let _elen = std::mem::size_of::<libc::c_int>() as u32;
			evtcall_log_trace!("error {} _elen {}",error,_elen);
			reti = libc::setsockopt(self.sock,libc::SOL_SOCKET,libc::SO_ERROR,_eptr,_elen);
		}

		if reti < 0 {
			reti = get_errno!();
			if reti != -libc::ENOPROTOOPT {
				evtcall_new_error!{SockHandleError,"setsockopt SO_ERROR error {}",reti}	
			}			
		}

		name = self._format_sockaddr_in(ipaddr,port)?;
		let mut inconn :i32 = 0;

		unsafe {
			let _nameptr = (&name as * const libc::sockaddr_in) as *const libc::sockaddr;
			let _namelen = std::mem::size_of::<libc::sockaddr_in>() as u32;
			reti = libc::connect(self.sock,_nameptr,_namelen);
		}
		if reti < 0 {
			reti = get_errno!();
			if reti != -libc::EINPROGRESS {
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
					libc::FD_SET(self.sock,_rdptr);
					let _nullptr = std::ptr::null_mut::<libc::fd_set>();
					let _timenull = std::ptr::null_mut::<libc::timeval>();
					reti = libc::select(self.sock + 1,_rdptr,_nullptr,_nullptr,_timenull);
				}

				if reti < 0 {
					reti = get_errno!();
					evtcall_new_error!{SockHandleError,"select error {}",reti}
				} else if reti == 0 {
					continue;
				} else {
					error = 1;
					unsafe {
						let _eptr = (&mut error as *mut libc::c_int) as *mut libc::c_void;
						let mut _elen = std::mem::size_of::<libc::c_int>() as u32;
						reti = libc::getsockopt(self.sock,libc::SOL_SOCKET,libc::SO_ERROR,_eptr,&mut _elen);
					}

					if reti < 0 {
						reti = get_errno!();
						evtcall_new_error!{SockHandleError,"getsockopt SO_ERROR error {}",reti}
					}
					if error != 0 {
						evtcall_new_error!{SockHandleError,"connect [{}:{}] error {}",ipaddr,port,error}
					}
					inconn = 0;
					self._get_sock_name()?;
					break;
				}
			}
		}
		self.inconn = inconn;
		Ok(())
	}

	pub (crate) fn connect_client(_ipaddr :&str,_port :u32,_localip :&str, _localport :u32, _connected :bool) -> Result<Arc<RefCell<Self>>,Box<dyn Error>> {
		let iretv :Self = Self::_default_new(TcpSockType::SockClientType);
		let retv = Arc::new(RefCell::new(iretv));
		Ok(retv)
	}

	pub (crate) fn get_accept_handle(&self) -> u64 {
		let mut retv :u64 = INVALID_EVENT_HANDLE;
		if self.inacc > 0 {
			retv = self.sock as u64;
		}
		return retv;
	}

	pub (crate) fn complete_connect(&mut self) -> Result<i32,Box<dyn Error>> {
		let mut completed :i32 = 1;
		match self.mtype {
			TcpSockType::SockClientType => {
				if self.inconn > 0 {
					let mut errval :libc::c_int = 1;
					let mut reti :i32;
					let mut check :bool = true;
					unsafe {
						let _eptr = (&mut errval as *mut libc::c_int) as *mut libc::c_void;
						let mut _errlen = std::mem::size_of::<libc::c_int>() as u32;
						reti = libc::getsockopt(self.sock,libc::SOL_SOCKET,libc::SO_ERROR,_eptr,&mut _errlen);
					}
					if reti < 0 {
						reti = get_errno!();
						if reti != -libc::EINPROGRESS {
							evtcall_new_error!{SockHandleError,"cannot get sockopt error {}",reti}
						}
						check = false;
					}
					if check {
						if errval != 0 {
							evtcall_new_error!{SockHandleError,"connect {}:{} error {}",self.peeraddr,self.peerport, errval}
						}
						self.inconn = 0;						
					}
				}
			},
			_ => {},
		}

		if self.inconn > 0 {
			completed = 0;
		}
		Ok(completed)
	}

	fn _inner_read(&mut self) -> Result<(),Box<dyn Error>> {
		let mut reti :isize;
		let erri :i32;
		loop {
			unsafe {
				let _rdptr = self.rdptr as *mut libc::c_char as * mut libc::c_void;
				let _rdlen = self.rdlen as usize;
				reti = libc::recv(self.sock,_rdptr,_rdlen,libc::MSG_DONTWAIT);
			}
			if reti < 0 {
				erri = get_errno!() ;
				if erri == - libc::EAGAIN || erri == -libc::EWOULDBLOCK {
					return Ok(());
				}

				evtcall_new_error!{SockHandleError,"read remote [{}:{}] => local [{}:{}] error {}", self.peeraddr,self.peerport,self.localaddr,self.localport,erri}
			}

			self.rdptr = unsafe{self.rdptr.offset(reti)};
			self.rdlen -= reti as u32;
			if self.rdlen == 0 {
				self.rdptr = std::ptr::null_mut::<u8>();
				self.inrd = 0;
				return Ok(());
			}
		}
	}

	pub (crate) fn complete_read(&mut self) -> Result<i32,Box<dyn Error>> {
		let mut completed :i32 = 1;
		if self.inrd > 0 {
			self._inner_read()?;
		}
		if self.inrd > 0 {
			completed = 0;
		}
		Ok(completed)
	}

	fn _inner_write(&mut self) -> Result<(),Box<dyn Error>> {
		let mut reti :isize;
		let erri :i32;
		loop {
			unsafe {
				let _wrptr = self.wrptr as *mut libc::c_char as * mut libc::c_void;
				let _wrlen = self.wrlen as usize;
				reti = libc::send(self.sock,_wrptr,_wrlen,libc::MSG_DONTWAIT);
			}
			if reti < 0 {
				erri = get_errno!() ;
				if erri == - libc::EAGAIN || erri == -libc::EWOULDBLOCK {
					return Ok(());
				}

				evtcall_new_error!{SockHandleError,"write local [{}:{}] => remote [{}:{}] error {}",self.localaddr,self.localport,self.peeraddr,self.peerport,erri}
			}

			self.wrptr = unsafe{self.wrptr.offset(reti)};
			self.wrlen -= reti as u32;
			if self.wrlen == 0 {
				self.wrptr = std::ptr::null_mut::<u8>();
				self.inwr = 0;
				return Ok(());
			}
		}
	}

	pub (crate) fn complete_write(&mut self) -> Result<i32,Box<dyn Error>> {
		let mut completed :i32 = 1;
		if self.inwr > 0 {
			self._inner_write()?;
		}
		if self.inwr > 0 {
			completed = 0;
		}
		Ok(completed)
	}

	pub (crate) fn read(&mut self,rbuf :*mut u8, rlen :u32) -> Result<i32,Box<dyn Error>> {
		if self.inrd > 0 || self.inacc > 0 || self.inconn > 0 || self.sock < 0 {
			evtcall_new_error!{SockHandleError,"invalid state for sock read"}
		}
		self.rdptr = rbuf;
		self.rdlen = rlen;
		self.inrd = 1;

		self._inner_read()?;
		let completed :i32;
		if self.inrd > 0 {
			completed = 0;
		} else {
			completed = 1;
		}
		Ok(completed)
	}

	pub (crate) fn write(&mut self,wbuf :*mut u8, wlen :u32) -> Result<i32,Box<dyn Error>> {
		if self.inwr > 0 || self.inacc > 0 || self.inconn > 0 || self.sock < 0 {
			evtcall_new_error!{SockHandleError,"invalid state for sock write"}
		}
		self.wrptr = wbuf;
		self.wrlen = wlen;
		self.inwr = 1;

		self._inner_write()?;
		let completed :i32;
		if self.inwr > 0 {
			completed = 0;
		} else {
			completed = 1;
		}
		Ok(completed)
	}

	pub (crate) fn get_read_handle(&self) -> u64 {
		let mut retv :u64 = INVALID_EVENT_HANDLE;
		if self.inrd > 0 {
			retv = self.sock as u64;
		}
		return retv;
	}

	pub (crate) fn get_write_handle(&self) -> u64 {
		let mut retv :u64 = INVALID_EVENT_HANDLE;
		if self.inwr > 0 {
			retv = self.sock as u64;
		}
		return retv;
	}

	pub (crate) fn close(&mut self) {
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

	pub (crate) fn is_accept_mode(&self) -> bool {
		let mut retv :bool = false;
		if self.inacc > 0 {
			retv = true;
		}
		return retv;
	}

	pub (crate) fn is_connect_mode(&self) -> bool {
		let mut retv :bool = false;
		if self.inconn > 0 {
			retv = true;
		}
		return retv;
	}

	pub (crate) fn is_read_mode(&self) -> bool {
		let mut retv :bool = false;
		if self.inrd > 0 {
			retv = true;
		}
		return retv;
	}

	pub (crate) fn is_write_mode(&self) -> bool {
		let mut retv :bool = false;
		if self.inwr > 0 {
			retv = true;
		}
		return retv;
	}

	pub (crate) fn get_self_format(&self) -> String {
		return format!("{}:{}",self.localaddr,self.localport);
	}

	pub (crate) fn get_peer_format(&self) -> String {
		return format!("{}:{}",self.peeraddr,self.peerport);
	}

	pub (crate) fn get_sock_real(&self) -> u64 {
		return self.sock as u64;
	}
}

impl TcpSockHandle {
	pub fn bind_server(ipaddr :&str,port :u32,backlog : i32) -> Result<Self,Box<dyn Error>> {
		let retv :Self = Self {
			inner : TcpSockHandleInner::bind_server(ipaddr,port,backlog)?,
		};
		retv.inner.borrow_mut().bind_server_after(ipaddr,port,backlog)?;
		Ok(retv)
	}

	pub fn complete_accept(&mut self) -> Result<i32,Box<dyn Error>> {
		return self.inner.borrow_mut().complete_accept();
	}

	pub fn accept_socket(&mut self) -> Result<Self,Box<dyn Error>> {
		let retv :Self = Self {
			inner : self.inner.borrow_mut().accept_socket()?,
		};
		Ok(retv)
	}

	pub fn connect_client(ipaddr :&str,port :u32,localip :&str, localport :u32, connected :bool) -> Result<Self,Box<dyn Error>> {
		let retv :Self = Self {
			inner : TcpSockHandleInner::connect_client(ipaddr,port,localip,localport,connected)?,
		};
		retv.inner.borrow_mut().connect_client_after(ipaddr,port,localip,localport,connected)?;
		Ok(retv)
	}

	pub fn get_accept_handle(&self) -> u64 {
		return self.inner.borrow().get_accept_handle();
	}

	pub fn complete_connect(&mut self) -> Result<i32,Box<dyn Error>> {
		return self.inner.borrow_mut().complete_connect();
	}

	pub fn complete_read(&mut self) -> Result<i32,Box<dyn Error>> {
		return self.inner.borrow_mut().complete_read();
	}

	pub fn complete_write(&mut self) -> Result<i32,Box<dyn Error>> {
		return self.inner.borrow_mut().complete_write();
	}

	pub fn read(&mut self,rbuf :*mut u8, rlen :u32) -> Result<i32,Box<dyn Error>> {
		return self.inner.borrow_mut().read(rbuf,rlen);
	}

	pub fn write(&mut self,wbuf :*mut u8, wlen :u32) -> Result<i32,Box<dyn Error>> {
		return self.inner.borrow_mut().write(wbuf,wlen);
	}

	pub fn get_read_handle(&self) -> u64 {
		return self.inner.borrow().get_read_handle();
	}

	pub fn get_write_handle(&self) -> u64 {
		return self.inner.borrow().get_write_handle();
	}

	pub fn close(&mut self) {
		self.inner.borrow_mut().close();
	}

	pub fn is_accept_mode(&self) -> bool {
		return self.inner.borrow().is_accept_mode();
	}

	pub fn is_connect_mode(&self) -> bool {
		return self.inner.borrow().is_connect_mode();
	}

	pub fn is_read_mode(&self) -> bool {
		return self.inner.borrow().is_read_mode();
	}

	pub fn is_write_mode(&self) -> bool {
		return self.inner.borrow().is_write_mode();
	}

	pub fn get_self_format(&self) -> String {
		return self.inner.borrow().get_self_format();
	}

	pub fn get_peer_format(&self) -> String {
		return self.inner.borrow().get_peer_format();
	}

	pub fn get_sock_real(&self) -> u64 {
		return self.inner.borrow().get_sock_real();
	}
}

pub fn init_socket() -> Result<(),Box<dyn Error>> {
	Ok(())
}

pub fn fini_socket()  {
}
