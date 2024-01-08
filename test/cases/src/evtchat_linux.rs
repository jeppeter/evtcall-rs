

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
		};
		Ok(retv)
	}

	pub fn get_handle(&self) -> u64 {
		return self.rd as u64;
	}

	pub fn read(&mut self, rdptr :*mut u8)  -> Result<i32,Box<dyn Error>> {
		let reti :i32;

		unsafe {
			reti = libc::read(self.rd,rdptr,1);
		}
		if reti < 0 {
			reti = get_errno!();
			if reti == -libc::EINTR || reti == -libc::EAGAIN || reti == -libc::EWOULDBLOCK {
				return Ok(0);
			}
			extargs_new_error!{EvtChatError,"can not read stdin error {}",reti}
		}
		Ok(1)
	}


	pub fn close(&mut self) {
		debug_trace!("close StdinRd");
		self.rd = -1;
	}
}

struct EvtChatClientInner {
	sock :TcpSockHandle,
	evttype :u32,
	insertsock : bool,
	exithd : u64,
	insertexit : bool,
	rdbuf :Vec<u8>,
	rdsidx : usize,
	rdeidx : usize,
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
	fn __inner_read(&mut self) -> Result<(),Box<dyn Error>> {
		
	}

	pub fn connect_client_after(&mut self,ipaddr :&str, port :u32,_timemills :i32, exithd :u64, _evtmain :&mut EvtMain,parent :EvtChatClient) -> Result<Arc<RefCell<Self>>,Box<dyn Error>> {
		unsafe {
			self.rdbuf.set_len(RDBUF_SIZE);
		}
		if self.sock.is_connect_mode() {
			self.evttype = READ_EVENT;
			_evtmain.add_event(Arc::new(RefCell::new(parent.clone())),self.sock.get_connect_handle(),self.evttype)?;
		} else {
			self.__inner_read()?;
		}

		Ok(())
	}

	pub fn connect_client(ipaddr :&str, port :u32,_timemills :i32, exithd :u64, _evtmain :&mut EvtMain) -> Result<Arc<RefCell<Self>>,Box<dyn Error>> {	
		let reti :Self = Self {
			sock : TcpSockHandle::connect_client(ipaddr,port,"",0,false)?,
			evttype : 0,
			insertsock : false,
			exithd : exithd,
			insertexit : false,
			rdbuf : Vec::new(),
			rdsidx : 0,
			rdeidx : 0,
		};
		let retv = Arc::new(RefCell::new(reti));
		Ok(retv)
	}
}