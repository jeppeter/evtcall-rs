

const RDBUF_SIZE : usize = 256;

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


struct StdinRd {
	rd :i32,
	inrd : bool,
}

impl Drop for StdinRd {
	fn drop(&mut self) {
		self.close();
	}
}

impl StdinRd {
	pub fn new() -> Result<Self,Box<dyn Error>> {
		let retv :Self = Self {
			rd : 0,
			inrd : false,
		};
		Ok(retv)
	}

	pub fn get_handle(&self) -> u64 {
		return self.rd as u64;
	}

	pub fn read(&mut self, rdptr :*mut u8)  -> Result<i32,Box<dyn Error>> {
		let reti :i32;

		if self.inrd {
			extargs_new_error!{EvtChatError,"inrd mode"}
		}

		unsafe {
			reti = libc::read(self.rd,rdptr,1);
		}
		if reti < 0 {
			reti = get_errno!();
			if reti == -libc::EINTR || reti == -libc::EAGAIN || reti == -libc::EWOULDBLOCK {
				self.inrd = true;
				return Ok(0);
			}
			extargs_new_error!{EvtChatError,"can not read stdin error {}",reti}
		}
		Ok(1)
	}

	pub fn is_read_mode(&self) -> bool {
		return self.inrd;
	}


	pub fn close(&mut self) {
		debug_trace!("close StdinRd");
		self.rd = -1;
	}
}

struct EvtChatClientInner {
	sock :TcpSockHandle,
	stdinrd :StdinRd,
	stdinfd :u64,
	sockfd :u64,
	exithd : u64,
	evttype :u32,
	insertsock : bool,
	insertexit : bool,
	insertstdin : bool,
	insertconntimeout : bool,
	connguid : u64,
	rdbuf :Vec<u8>,
	rdsidx : usize,
	rdeidx : usize,
	rdlen : usize,

	stdinrdbuf :Vec<u8>,
	stdinrdsidx : usize,
	stdinrdeidx : usize,
	stdinrdlen : usize,

	wbuf : Vec<u8>,
	wbufs :Vec<Vec<u8>>,
}

struct EvtChatClient {
	inner :Arc<RefCell<EvtChatClientInner>>,
}

impl Drop for EvtChatClientInner {
	fn drop(&mut self) {
		self.close();
	}
}

impl Drop for EvtChatClient {
	fn drop(&mut self) {
		self.close();
	}
}


impl EvtChatClientInner {

	fn __write_stdout(&mut self) -> Result<(),Box<dyn Error>> {
		let mut wbuf :Vec<u8> = Vec::new();
		let mut idx :usize = 0;
		let mut cidx :usize;

		while idx < self.rdlen {
			cidx = self.rdsidx + idx;
			cidx %= self.rdbuf.len();
			wbuf.push(self.rdbuf[cidx]);
			idx += 1;
		}
		self.rdlen = 0;
		self.rdsidx = self.rdeidx;

		if wbuf.len() > 0 {
			let s :String = String::from_utf8_lossy(&wbuf).to_string();
			let mut of = std::io::stdout();
			of.write(s.as_bytes())?;
			of.flush()?;
			Ok(())			
		}

		Ok(())
	}

	fn __inner_stdin_read(&mut self,parent :EvtChatClient) -> Result<(),Box<dyn Error>> {
		if !self.stdinrd.is_read_mode() {
			loop {
				if self.stdinrdlen == self.stdinrdbuf.len() {
					self.__inner_sock_write(parent.clone())?;
				}

				let completed = self.stdinrd.read(&self.stdinrdbuf[self.stdinrdeidx])?;
				if completed == 0 {
					break;
				}
				self.stdinrdeidx += 1;
				self.stdinrdeidx %= self.stdinrdbuf.len();
				self.stdinrdlen += 1;
			}
		}

		if !self.insertstdinrd {
			self.stdinfd = self.stdinrd.get_handle();
			unsafe {
				(*self.evmain).add_event(Arc::new(RefCell::new(parent.clone())),self.stdinfd,READ_EVENT)?;
			}
			self.insertstdinrd = true;
		}
		Ok(())
	}	

	fn __inner_sock_read(&mut self,parent :EvtChatClient) -> Result<(),Box<dyn Error>> {
		if !self.insertsock {
			loop {
				if self.rdlen == self.rdbuf.len() {
					self.__write_stdout()?;
				}

				let completed = self.sock.read(&mut self.rdbuf[self.rdeidx],1)?;
				if completed == 0 {
					self.__write_stdout()?;
					break;
				}

				self.rdeidx += 1;
				self.rdeidx %= self.rdbuf.len();
				self.rdlen += 1;
			}
		}

		if !self.insertsock {
			self.evttype |= READ_EVENT;
			self.sockfd = self.sock.get_sock_real();
			unsafe {
				(*self.evmain).add_event(Arc::new(RefCell::new(parent.clone())),self.sockfd,self.evttype)?;
			}
			self.insertsock = true;
		}
		self.inrd = true;

		Ok(())
	}

	fn __stdin_write_sock(&mut self, parent :EvtChatClient) ->Result<(),Box<dyn Error>> {
		let mut wbuf :Vec<u8> = Vec::new();
		let mut idx :usize = 0;
		let mut cidx :usize;

		while idx < self.stdinrdlen {
			cidx = self.stdinrdsidx + idx;
			cidx %= self.stdinrdbuf.len();
			wbuf.push(self.stdinrdbuf[cidx]);
			idx += 1;
		}
		self.stdinrdsidx = self.stdinrdeidx;
		self.stdinrdlen = 0;

		if self.wbuf.len() == 0 {
			self.wbuf = wbuf;
		} else {
			self.wbufs.push(wbuf);
		}

		return self.__inner_sock_write(parent);
	}

	fn __inner_sock_write(&mut self,parent :EvtChatClient) -> Result<(),Box<dyn Error>> {

		if !self.insertsock && (self.evttype & WRITE_EVENT) == 0 {
			loop {
				if self.wbuf.len() > 0 {
					let completed = self.sock.write(self.wbuf.as_mut_ptr(),self.wbuf.len())?;
					if completed == 0 {
						self.evttype |= WRITE_EVENT;
						break;
					}
					self.wbuf = Vec::new();
				}

				if self.wbufs.len() > 0 {
					self.wbuf = self.wbufs[0].clone();
					self.wbufs.remove(0);
				}

				if self.wbuf.len() == 0 && self.wbufs.len() == 0 {
					self.evttype &= ~WRITE_EVENT;
					break;
				}	

				/*now to continue*/
			}
		}

		if !self.insertsock && self.wbuf.len() > 0 {
			self.sockfd = self.sock.get_sock_real();
			unsafe {
				(*self.evmain).add_event(Arc::new(RefCell::new(parent.clone())),self.sockfd,self.evttype)?;
			}
			self.insertsock = true;
		}

		return Ok(());
	}

	pub fn connect_client_after(&mut self,timemills :i32,  parent :EvtChatClient) -> Result<Arc<RefCell<Self>>,Box<dyn Error>> {
		unsafe {
			self.rdbuf.set_len(RDBUF_SIZE);
			self.stdinrdbuf.set_len(RDBUF_SIZE);
		}
		if self.sock.is_connect_mode() {
			self.evttype = READ_EVENT;
			self.sockfd = self.sock.get_sock_real();
			unsafe {
				(*self.evmain).add_event(Arc::new(RefCell::new(parent.clone())),self.sockfd,self.evttype)?;	
			}
			
			self.insertsock = true;

			unsafe {
				self.connguid =(*self.evmain).add_timer(Arc::new(RefCell::new(parent.clone())),timemills,false)?;	
			}
			
			self.insertconntimeout = true;
		} else {
			self.__inner_sock_read(parent.clone())?;
			self.__inner_stdin_read(parent.clone())?;
		}

		if !self.insertexit {
			unsafe {
				(*self.evmain).add_event(Arc::new(RefCell::new(parent.clone())),self.exithd,READ_EVENT)?;	
			}			
			self.insertexit = true;
		}
		Ok(())
	}

	pub fn connect_client(ipaddr :&str, port :u32,_timemills :i32, exithd :u64, _evtmain :&mut EvtMain) -> Result<Arc<RefCell<Self>>,Box<dyn Error>> {	
		let reti :Self = Self {
			sock : TcpSockHandle::connect_client(ipaddr,port,"",0,false)?,
			sockfd : INVALID_EVENT_HANDLE,
			stdinfd : INVALID_EVENT_HANDLE,
			inrd : false,
			evttype : 0,
			insertsock : false,
			exithd : exithd,
			insertexit : false,
			connguid : 0,
			insertconntimeout : false,
			rdbuf : Vec::new(),
			rdsidx : 0,
			rdeidx : 0,
			rdlen : 0,

			stdinrdbuf : Vec::new(),
			stdinrdsidx : 0,
			stdinrdeidx : 0,
			stdinrdlen : 0,

			wbuf : Vec::new(),
			wbufs : Vec::new(),
		};
		let retv = Arc::new(RefCell::new(reti));
		Ok(retv)
	}

	pub fn read_sock_handle(&mut self,evthd :u64 ,evttype :u32, evtmain :&mut EvtMain,parent : EvtChatClient) -> Result<(),Box<dyn Error>> {
		if self.sock.is_read_mode() {
			let completed = self.sock.complete_read()?;
			if completed == 0 {
				return Ok(());
			}
			evtmain.remove_event(self.sockfd)?;
			self.insertsock = false;

			self.__inner_read(parent.clone())?;
		}
		Ok(())
	}

	pub fn handle(&mut self, evthd :u64, evttype :u32,evtmain :&mut EvtMain,parent :EvtChatClient) -> Result<(),Box<dyn Error>> {
		if evthd == self.sockfd && (evttype & READ_EVENT) != 0 {

			if (evttype & READ_EVENT) != 0 {
				if !self.inrd {
					let completed = self.sock.complete_connect()?;
					if completed > 0 {
						evtmain.remove_event(self.sockfd);
						self.insertsock = false;
						evtmain.remove_timer(self.connguid);
						self.insertconntimeout = false;
						self.__inner_sock_read(parent.clone())?
						self.__inner_stdin_read(parent.clone())?;
					}
				} else {				
					let completed = self.sock.complete_read()?;
					if completed > 0 {
						evtmain.remove_event(self.sockfd);
						self.insertsock = false;
						self.__inner_sock_read(parent.clone())?;
					}
				}
				assert!(self.insertsock);

			}  
			if (evttype & WRITE_EVENT) != 0 {
				let completed = self.sock.complete_write()?;
				if completed > 0  {
					evtmain.remove_event(self.sockfd);
					self.insertsock = false;
					self.wbuf = Vec::new();
					self.__inner_sock_write(parent.clone())?;
					if !self.insertsock && self.evttype != 0 {
						self.sockfd = self.sock.get_sock_real();
						evtmain.add_event(Arc::new(RefCell::new(parent.clone())),self.sockfd,self.evttype)?;
					}
				}
				assert!(self.insertsock);
			}
		} else if evthd == self.stdinfd {
			if (evttype & READ_EVENT ) != 0 {
				self.__inner_stdin_read(parent.clone())?;
			}
		} else if evthd == self.exithd {
			evtmain.break_up()?;
		}


		Ok(())
	}

	fn __close_event_inner(&mut self)  {
		if self.insertsock {
			(*self.evmain).remove_event(self.sockfd);
			self.insertsock = false;
		}

		if self.insertstdinrd {
			(*self.evmain).remove_event(self.stdinfd);
			self.insertstdinrd = false;
		}

		if self.insertexit {
			(*self.evmain).remove_event(self.exithd);
			self.insertexit = false;
		}

		return;
	}

	fn __close_timer_inner(&mut self) {
		if self.insertconntimeout {
			(*self.evmain).remove_timer(self.connguid);
			self.insertconntimeout = false;
		}
		return ;
	}



	pub fn close(&mut self)  {
		self.__close_event_inner();
		self.__close_timer_inner();

		assert!(!self.insertconntimeout);
		assert!(!self.insertsock);
		assert!(!self.insertexit);
		assert!(!self.insertstdinrd);
		self.sock.close();

		self.rdsidx = 0;
		self.rdeidx = 0;
		self.rdlen = 0;
		self.rdbuf = Vec::new();

		self.stdinrdsidx = 0;
		self.stdinrdeidx = 0;
		self.stdinrdlen = 0;
		self.stdinrdbuf = Vec::new();


		self.wbuf = Vec::new();
		self.wbufs = Vec::new();
	}

	pub fn debug_mode(&mut self,_fname :&str, _lineno :u32) {
		return;
	}

	pub fn close_event(&mut self,_evthd :u64, _evttype :u32,_evtmain :&mut EvtMain) {
		self.__close_event_inner();
	}

	pub fn close_timer(&mut self,, _guid :u64, _evtmain :&mut EvtMain) {
		self.__close_timer_inner();
	}

}

impl EvtCall for EvtChatClient {
	fn debug_mode(&mut self,fname :&str, lineno :u32) {
		return self.inner.borrow_mut().debug_mode(fname,lineno,self.clone());
	}

	fn handle(&mut self,evthd :u64, evttype :u32,evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>> {
		let p :EvtChatClient = self.clone();
		return self.inner.borrow_mut().handle(evthd,evttype,evtmain,p);
	}

	fn close_event(&mut self,evthd :u64, evttype :u32,evtmain :&mut EvtMain) {
		let p :EvtChatClient = self.clone();
		return self.inner.borrow_mut().close_event();
	}
}

impl EvtTimer for EvtChatClient {
	fn timer(&mut self,_timerguid :u64,_evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>> {
		let p :EvtChatClient = self.clone();
		return self.inner.borrow_mut().timer(_timerguid,_evtmain,p);
	}

	fn close_timer(&mut self, _guid :u64, _evtmain :&mut EvtMain) {
		let p :EvtChatClient = self.clone();
		return self.inner.borrow_mut().close_timer();
	}
}

impl EvtChatClient {
	pub fn connect_client(ipaddr :&str, port :u32,timemills :i32, exithd :u64, evtmain :&mut EvtMain) -> Result<Self,Box<dyn Error>> {	
		let ninner :Arc<RefCell<EvtChatClientInner>> = EvtChatClientInner::connect_client(ipaddr,port,exithd,evtmain)?;
		let retv :Self = Self {
			inner :ninner,
		};
		retv.inner.borrow_mut().connect_client_after(timemills,retv.clone())?;
		Ok(retv)
	}

	pub fn close(&mut self) {
		/*we do not close this*/
		debug_trace!("EvtChatClient close {:p}",self);
	}
}

struct EvtChatServerConnInner {
	sock :TcpSockHandle,
	sockfd :u64,
	evttype :u32,
	insertsock : bool,
	inrd : bool,
	inwr : bool,
	evmain :*mut EvtMain,
	svr :* mut EvtChatServerInner,
	rdbuf :Vec<u8>,
	rdsidx : usize,
	rdeidx : usize,
	rdlen : usize,

	wbuf : Vec<u8>,
	wbufs : Vec<Vec<u8>>,
}

#[derive(Clone)]
struct EvtChatServerConn {
	inner :Arc<RefCell<EvtChatServerConnInner>>,
}

impl EvtChatServerConnInner {

	fn _inner_write(&mut self) -> Result<(),Box<dyn Error>> {
		if !self.inwr {
			loop {
				if self.wbuf.len() > 0 {
					let completed = self.sock.write(self.wbuf.as_mut_ptr(),self.wbuf.len())?;
					if completed == 0 {
						self.inwr = true;
						break;
					}
					self.wbuf = Vec::new();
				}

				if self.wbufs.len() > 0 {
					self.wbuf = self.wbufs[0].clone();
					self.wbufs.remove(0);
				}

				if self.wbuf.len() == 0 {
					break;
				}
			}
		}
		Ok(())
	}

	fn _insert_sock(&mut self,parent :EvtChatServerConn) -> Result<(),Box<dyn Error>> {
		let mut willinsert :bool =false;
		let mut willremove :bool = false;
		if self.inwr || self.inrd {
			if !self.insertsock {
				willinsert = true;
			}

			if self.inwr && (self.evttype & WRITE_EVENT) == 0 {
				willinsert = true;
			}

			if self.inrd && (self.evttype & READ_EVENT) == 0 {
				willinsert = true;
			}
		}

		if !self.inwr && !self.inrd {
			willremove = true;
		}

		if willinsert {
			self.evttype = 0;
			if self.inrd {
				self.evttype |= READ_EVENT;
			}
			if self.inwr {
				self.evttype |= WRITE_EVENT;
			}
			if self.insertsock {
				unsafe {
					(*self.evmain).remove_event(self.sockfd);
				}
				self.insertsock = false;
			}

			self.sockfd = self.sock.get_sock_real();
			unsafe {
				(*self.evmain).add_event(Arc::new(RefCell::new(parent.clone())),self.sockfd,self.evttype)?;
			}
			self.insertsock = true;
		}

		if willremove {
			if self.insertsock {
				unsafe {
					(*self.evmain).remove_event(self.sockfd);
				}
				self.insertsock = false;
			}
		}

		Ok(())
	}

	fn _echo_write(&mut self) -> Result<(),Box<dyn Error>> {
		let mut wbuf :Vec<u8> = Vec::new();
		let mut idx :usize = 0;
		let mut cidx :usize;

		while idx < self.rdlen {
			cidx = self.rdsidx + idx;
			cidx %= self.rdbuf.len();
			wbuf.push(self.rdbuf[cidx]);
			idx += 1;
		}

		if wbuf.len() == 0 {
			return Ok(());
		}

		if self.wbuf.len() == 0 {
			self.wbuf = wbuf;
		} else {
			self.wbufs.push(wbuf);
		}
		self._inner_write()?;
		Ok(())
	}

	fn _inner_read(&mut self,parent :EvtChatServerConn) -> Result<(),Box<dyn Error>> {
		let mut mustinsert : bool = false;
		if !self.inrd {
			loop {
				if self.rdlen == self.rdbuf.len() {
					self._echo_write()?;
				}

				let completed = self.sock.read(&mut self.rdbuf[self.rdeidx],1)?;
				if completed == 0 {
					self._echo_write()?;
					break;
				}
				self.rdeidx += 1;
				self.rdeidx %= self.rdbuf.len();
				self.rdlen += 1;
			}
			self.inrd = true;
		}
		Ok(())
	}

	pub (crate) new_after(&mut self,parent :EvtChatServerConn) -> Result<(),Box<dyn Error>> {
		unsafe {
			self.rdbuf.set_len(RDBUF_SIZE);
		}
		self._inner_read(parent.clone())?;
		self._insert_sock(parent.clone())?;
		Ok(())
	}

	pub (crate) new(sock :TcpSockHandle,svr :*mut EvtChatServerConnInner,evmain :*mut EvtMain) -> Result<Self,Box<dyn Error>> {
		let mut reti :Self = Self{
			sock : sock,
			sockfd : INVALID_EVENT_HANDLE,
			insertsock : false,
			evttype : 0,
			evmain : evmain,
			svr : svr,	
			rdbuf : Vec::new(),
			rdsidx : 0,
			rdeidx : 0,
			rdlen : 0,

			wbuf : Vec::new(),
			wbufs : Vec::new(),
		};
		Ok(Arc::new(RefCell::new(reti)))
	}

	pub (crate) handle(&mut self, evthd :u64,evttype :u32,evtmain :&mut EvtMain,parent :EvtChatServerConn) -> Result<(),Box<dyn Error>> {
		if evthd == self.sockfd && (evttype & READ_EVENT) != 0 {
			let completed = self.sock.complete_read()?;
			if completed > 0 {
				self.inrd = false;
				self._inner_read()?;
			}
		}

		if evthd == self.sockfd && (evttype & WRITE_EVENT) != 0 {
			let completed = self.sock.complete_write()?;
			if completed > 0 {
				self.inwr = false;
				self._inner_write()?;
			}
		}

		self._insert_sock(parent.clone())?;
		Ok(())
	}

	fn __close_event_inner(&mut self) {
		if self.insertsock {
			unsafe {
				(*self.evmain).remove_event(self.sockfd);
			}
			self.insertsock = false;
		}
		return;
	}

	pub (crate) close_event(&mut self, evthd :u64, evttype :u32,evmain :&mut EvtMain) ->Result<(),Box<dyn Error>> {
		self.__close_event_inner();
	}

	pub (crate) close(&mut self) {
		self.__close_event_inner();
		self.sockfd = self.sock.get_sock_real();
		unsafe {
			self.svr.remove_connection(self.sockfd);
		}		
		self.sock.close();
		self.rdbuf = Vec::new();
		self.rdsidx = 0;
		self.rdeidx = 0;
		self.rdlen = 0;
		self.wbuf = Vec::new();
		self.wbufs = Vec::new();
	}
}

impl EvtChatServerConn {
	pub (crate) fn new(sock :TcpSockHandle,svr :*mut EvtChatServerInner,evmain :*mut EvtMain) -> Result<Self,Box<dyn Error>> {
		let ninner  = EvtChatServerInner::new(sock,svr,evmain)?;
		let retv :Self = Self {
			inner :ninner,
		};
		let p :Self = retv.clone();
		retv.inner.borrow_mut().new_after(retv)?;
		Ok(retv)
	}
}

impl EvtCall for EvtChatServerConn {
	fn debug_mode(&mut self,fname :&str, lineno :u32) {
		return self.inner.borrow_mut().debug_mode(fname,lineno);
	}
	fn handle(&mut self,evthd :u64, evttype :u32,evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>> {
		let p = self.clone();
		return self.inner.borrow_mut().handle(evthd,evttype,evtmain,p);
	}
	fn close_event(&mut self,evthd :u64, evttype :u32,evtmain :&mut EvtMain) {
		self.inner.borrow_mut().close_event(evthd,evttype,evtmain);
		return;
	}
}


struct EvtChatServerInner {
	sock :TcpSockHandle,
	sockfd : u64,
	insertsock : bool,
	insertexit : bool,


	accsocks : Vec<EvtChatServerConn>,
	evmain :*mut EvtMain,
}

struct EvtChatServer {
	inner : Arc<RefCell<EvtChatServerInner>>,
}

impl Drop for EvtChatServerInner {
	fn drop(&mut self) {
		self.close();
	}
}

impl EvtChatServerInner {
	pub (crate) fn bind_server(ipaddr :&str, port :u32,backlog :i32,exithd :u64, evtmain :*mut EvtMain) -> Result<Arc<RefCell<Self>>,Box<dyn Error>> {
		let iretv :Self = Self {
			sock : TcpSockHandle::bind_server(ipaddr,port,backlog)?,
			evmain : evtmain,
			accsocks : Vec::new(),
			exithd : exithd,
			insertexit : false,
			insertsock : false,
			evttype : 0,
			sockfd  : INVALID_EVENT_HANDLE,
		};
		let retv = Arc::new(RefCell::new(iretv));
		Ok(retv)
	}

	fn __close_event_inner(&mut self) {
		if self.insertsock {
			unsafe {
				(*self.evmain).remove_event(self.sockfd);
			}
			self.insertsock = false;
		}

		if self.insertexit {
			unsafe {
				(*self.evmain).remove_event(self.exithd);
			}
			self.insertexit = false;
		}
		return;
	}

	pub (crate) fn close_event(&mut self,evthd :u64, evttype :u32, evtmain :&mut EvtMain) {
		self.__close_event_inner();
		return;
	}

	fn _inner_accept(&mut self, parent : EvtChatServer) -> Result<(),Box<dyn Error>> {
		if !self.insertsock {
			loop {
				let nsock :TcpSockHandle = self.sock.accept_socket()?;
				let nconn :EvtChatServerConn = EvtChatServerConn::new(nsock,self as *mut EvtChatServerConnInner,self.evmain )?;
				self.accsocks.push(nconn);
				if self.sock.is_accept_mode() {
					break;
				}
				let completed = self.sock.complete_accept()?;
				if completed == 0 {
					break;
				}
			}
			unsafe {
				(*self.evmain).add_event(Arc::new(RefCell::new(parent.clone())),self.sockfd,READ_EVENT)?;
			}
			self.insertsock = true;
		}		
		Ok(())
	}

	pub (crate) fn bind_server_after(&mut self,parent :EvtChatServer) -> Result<(),Box<dyn Error>> {
		if !self.sock.is_accept_mode() {
			self._inner_accept(parent.clone())?;
		} else {
			self.sockfd = self.sock.get_sock_real();
			unsafe {
				(*self.evmain).add_event(Arc::new(RefCell::new(parent.clone())),self.sockfd,READ_EVENT)?;
			}
			self.insertsock = true;
		}
		unsafe {
			(*self.evmain).add_event(Arc::new(RefCell::new(parent.clone())),self.exithd,READ_EVENT)?;
		}
		self.insertexit = true;
		Ok(())
	}

	pub (crate) fn handle(&mut self, evthd :u64,evttype :u32, evmain :&mut EvtMain,parent :EvtChatServer) -> Result<(),Box<dyn Error>> {
		if evthd == self.sockfd {
			let completed = self.sock.complete_accept()?;
			if completed {
				evmain.remove_event(self.sockfd);
				self.insertsock = false;
				self._inner_accept()?;
			}
		} else if evthd == self.exithd {
			evmain.break_up()?;
		} else {
			extargs_new_error!{EvtChatError,"not support 0x{:x} evthd ",evthd}
		}
		Ok(())
	}

	pub (crate) fn remove_connection(&mut self, fd :u64) -> i32 {
		let mut idx :usize = 0;
		let mut fidx : i32 = -1;
		let mut reti :i32 = 0;

		while idx < self.accsocks.len() {
			if self.accsocks[0].get_sock_real() == fd {
				fidx = idx as i32;
				break;
			}
			idx += 1;
		}

		if fidx >= 0 {
			self.accsocks.remove(fidx as usize);
			reti = 1;
		}
		return reti;
	}

	pub (crate) fn close(&mut self) {
		self.__close_event_inner();
		assert!(!self.insertsock);
		assert!(!self.insertexit);
		while self.accsocks.len() > 0 {
			self.accsocks.remove(0);
		}
		assert!(self.accsocks.len() == 0);
		self.sock.close();
	}
}

impl EvtChatServer {
	pub fn bind_server(ipaddr :&str, port :u32,backlog :i32,exithd :u64, evtmain :*mut EvtMain) -> Result<Self,Box<dyn Error>> {
		let ninner = EvtChatServerInner::bind_server(ipaddr,port,backlog,exithd,evtmain)?;
		let retv :Self = Self {
			inner :ninner,
		};
		let p = retv.clone();
		{
			debug_trace!("borrow_mut for bind_server_after {:p}",&retv);
			retv.inner.borrow_mut().bind_server_after(p)?;
		}
		debug_trace!("free_mut for bind_server_after {:p}",&retv);
		
		Ok(retv)
	}

	pub fn close(&mut self) {
		debug_trace!("EvtChatServer close {:p}",self);
		return;
	}
}

impl EvtCall for EvtChatServer {
	fn handle(&mut self,evthd :u64, _evttype :u32,evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>> {
		let p = self.clone();
		return self.inner.borrow_mut().handle(evthd,_evttype,evtmain,p);
	}

	fn debug_mode(&mut self,fname :&str, lineno :u32) {
		let p = self.clone();
		return self.inner.borrow_mut().debug_mode(fname,lineno,p);
	}

	fn close_event(&mut self,_evthd :u64, _evttype :u32, _evtmain :&mut EvtMain)  {
		let p = self.clone();
		return self.inner.borrow_mut().close_event(_evthd,_evttype,_evtmain,p);
	}
}
