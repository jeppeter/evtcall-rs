
use winapi::um::winnt::{HANDLE};
use winapi::um::processenv::{GetStdHandle};
use winapi::um::winbase::{STD_INPUT_HANDLE};
use winapi::um::consoleapi::{PeekConsoleInputA,ReadConsoleInputA};
use winapi::um::wincontypes::{INPUT_RECORD,KEY_EVENT};
use winapi::um::errhandlingapi::{GetLastError};
use winapi::shared::minwindef::{DWORD,TRUE,FALSE,BOOL};
use winapi::um::handleapi::{CloseHandle};


const RDBUF_SIZE : usize = 256;

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



struct StdinRd {
	rd :HANDLE,
}

impl Drop for StdinRd {
	fn drop(&mut self) {
		self.close();
	}
}

impl StdinRd {
	pub fn new() -> Result<Self,Box<dyn Error>> {
		let retv :Self = Self {
			rd : unsafe {GetStdHandle(STD_INPUT_HANDLE)},
		};
		Ok(retv)
	}

	pub fn get_handle(&self) -> u64 {
		return self.rd as u64;
	}

	pub fn read(&mut self,rdptr :*mut u8) -> Result<i32,Box<dyn Error>> {
		let mut bret :BOOL;
		let mut ir :INPUT_RECORD = unsafe{std::mem::zeroed()};
		let retval :i32;
		loop {
			unsafe {
				let _irptr = &mut ir as *mut INPUT_RECORD;
				let mut _dcnt :DWORD = 0;
				bret = PeekConsoleInputA(self.rd,_irptr,1,&mut _dcnt);
			}
			if bret == FALSE {
				return Ok(0);
			}

			unsafe {
				let _irptr = &mut ir as *mut INPUT_RECORD;
				let mut _dcnt :DWORD = 0;
				bret = ReadConsoleInputA(self.rd,_irptr,1,&mut _dcnt);
			}
			if bret == FALSE {
				retval = get_errno!();
				extargs_new_error!{EvtChatError,"can not ReadConsoleInputA error {}",retval}
			}

			if ir.EventType == KEY_EVENT &&  unsafe{ir.Event.KeyEvent().bKeyDown} == TRUE {
				if unsafe{*ir.Event.KeyEvent().uChar.AsciiChar()} != 0 {
					unsafe {*rdptr = *ir.Event.KeyEvent().uChar.AsciiChar() as u8};
					return Ok(1);
				}
			}
		}
	}

	pub fn close(&mut self) {
		let bret :BOOL;
		if self.rd != INVALID_EVENT_HANDLE as HANDLE {
			unsafe {
				bret = CloseHandle(self.rd);
			}
			if bret == FALSE {
				let reti :i32 = get_errno!();
				debug_error!("CloseHandle rd error {}",reti);
			}
		}
		self.rd = INVALID_EVENT_HANDLE as HANDLE;
	}
}


struct EvtChatClient {
	sock :TcpSockHandle,
	connhd : u64,
	rdhd :u64,
	wrhd :u64,
	rdvecs :Vec<u8>,
	rdsidx : usize,
	rdeidx : usize,
	rdlen : usize,
	insertconn : i32,
	insertrd  :i32,
	insertwr :i32,
	sockwbuf :Vec<u8>,
	sockwbufs :Vec<Vec<u8>>,
	connguid :u64,
	insertconntimeout : i32,

	stdinrd : StdinRd,
	stdinrdhd : u64,
	stdinvecs :Vec<u8>,
	stdinrdsidx : usize,
	stdinrdeidx : usize,
	stdinrdlen :usize,
	insertstdinrd : i32,


	evmain:*mut EvtMain,
}

impl Drop for EvtChatClient {
	fn drop(&mut self) {
		self.close();
	}
}

impl EvtCall for EvtChatClient {
	fn handle(&mut self,evthd :u64, _evttype :u32,_evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>> {
		if evthd == self.sock.get_connect_handle() {
			self.connect_handle()?;
		} else if evthd == self.sock.get_write_handle() {
			self.sock_write_proc()?;
		} else if evthd == self.sock.get_read_handle() {
			self.sock_read_proc()?;
		} else if evthd == self.stdinrd.get_handle() {
			self.stdin_read_proc()?;
		} else {
			extargs_new_error!{EvtChatError,"not recognize evthd 0x{:x}",evthd}
		}
		Ok(())
	}	

	fn close_event(&mut self,_evthd :u64, _evttype :u32, _evtmain :&mut EvtMain)  {
		self.close_event_inner();
		return;
	}
}

impl EvtTimer for EvtChatClient {
	fn timer(&mut self,_timerguid :u64,_evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>> {
		return self.connect_timeout();
	}

	fn close_timer(&mut self, _guid :u64, _evtmain :&mut EvtMain) {
		self.close_timer_inner();
		return;
	}
}


impl EvtChatClient {

	fn close_event_inner(&mut self) {
		self.stdinrd.close();
		self.sock.close();

		if self.insertconn > 0 {
			unsafe {
				let _ = &(*self.evmain).remove_event(self.connhd);
			}
			self.insertconn = 0;
		}

		if self.insertrd > 0 {
			unsafe {
				let _ = &(*self.evmain).remove_event(self.rdhd);
			}
			self.insertrd = 0;
		}

		if self.insertwr > 0 {
			unsafe {
				let _ = &(*self.evmain).remove_event(self.wrhd);
			}
			self.insertwr = 0;
		}

		if self.insertstdinrd > 0 {
			unsafe {
				let _ = &(*self.evmain).remove_event(self.stdinrdhd);
			}
			self.insertstdinrd = 0;
		}

		self.check_clear_evmain();
		return;
	}
	fn close_timer_inner(&mut self) {
		if self.insertconntimeout > 0 {
			unsafe {
				let _ = &(*(self.evmain)).remove_timer(self.connguid);
			}
			self.insertconntimeout = 0;
		}
		self.check_clear_evmain();
		return;		
	}

	fn _conti_write_sock(&mut self) -> Result<(),Box<dyn Error>> {
		let mut completed :i32 = 0;
		if self.insertwr == 0 {
			loop {
				if self.sockwbuf.len() > 0 {
					let completed = self.sock.write(self.sockwbuf.as_mut_ptr(),self.sockwbufs.len() as u32)?;
					if completed > 0 {
						self.sockwbuf = Vec::new();
					} else {
						self.wrhd = self.sock.get_write_handle();
						unsafe {
							let _ = &(*self.evmain).add_event(Arc::new( self as *mut dyn EvtCall),self.wrhd,WRITE_EVENT)?;
						}
						self.insertwr = 1;
						return Ok(());
					}
				}

				if self.sockwbufs.len() == 0 {
					completed = 1;
					break;
				}
				self.sockwbuf = self.sockwbufs[0].clone();
				self.sockwbufs.remove(0);
			}
		}

		if completed > 0 {
			if self.insertwr > 0 {
				unsafe {
					let _ = &(*self.evmain).remove_event(self.wrhd);
				}
				self.insertwr = 0;
			}
		}
		Ok(())
	}

	fn _sock_write_inner(&mut self) -> Result<(),Box<dyn Error>> {
		let mut wbuf :Vec<u8> = Vec::new();
		let mut idx :usize =0;
		let mut curidx :usize;
		let completed :i32;
		while idx < self.stdinrdlen {
			curidx = self.stdinrdsidx + idx;
			curidx %= self.stdinvecs.len();
			wbuf.push(self.stdinvecs[curidx]);
			idx += 1;
		}
		self.stdinrdsidx = self.stdinrdeidx;
		self.stdinrdlen = 0;

		if self.sockwbuf.len() == 0 {
			self.sockwbuf = wbuf.clone();
			completed = self.sock.write(self.sockwbuf.as_ptr() as *mut u8,self.sockwbuf.len() as u32)?;
			if completed > 0 {
				if self.insertwr > 0 {
					assert!(self.wrhd != INVALID_EVENT_HANDLE);
					unsafe {
						let _ = &(*self.evmain).remove_event(self.wrhd);
					}
					self.insertwr = 0;
				}
				self.sockwbuf = Vec::new();
			} else {
				if self.insertwr == 0 {
					self.wrhd = self.sock.get_write_handle();
					unsafe {
						let _ = &(*self.evmain).add_event(Arc::new(self),self.wrhd,WRITE_EVENT)?;
					}
					self.insertwr = 1;
				}
			}
		} else {
			self.sockwbufs.push(wbuf);
		}
		self._conti_write_sock()?;
		Ok(())

	}

	pub fn stdin_read_proc(&mut self) -> Result<(),Box<dyn Error>> {
		loop {
			if self.rdlen == self.rdvecs.len() {
				self._sock_write_inner()?;
			}

			let _rptr = (&mut self.stdinvecs[self.stdinrdeidx]) as *mut u8;
			let completed = self.stdinrd.read(_rptr)?;
			if completed > 0 {
				self.stdinrdeidx += 1;
				self.stdinrdeidx %= self.stdinvecs.len();
				self.stdinrdlen += 1;
			} else {
				if self.insertstdinrd == 0 {
					self.stdinrdhd = self.stdinrd.get_handle();
					unsafe {
						let _ = &(*self.evmain).add_event(Arc::new(self),self.stdinrdhd,READ_EVENT)?;
					}
					self.insertstdinrd = 1;
				}
				self._sock_write_inner()?;
				return Ok(());
			}
		}
	}

	fn _write_stdout_inner(&mut self) -> Result<(),Box<dyn Error>> {
		let mut rdvecs :Vec<u8> = Vec::new();
		let mut idx :usize = 0;
		let mut curidx :usize;
		while idx < self.rdlen {
			curidx = self.rdsidx + idx;
			curidx %= self.rdvecs.len();
			rdvecs.push(self.rdvecs[curidx]);
			idx += 1;
		}
		self.rdsidx = self.rdeidx;
		self.rdlen = 0;
		let s :String = String::from_utf8_lossy(&rdvecs).to_string();
		let mut of = std::io::stdout();
		of.write(s.as_bytes())?;
		of.flush()?;
		Ok(())
	}
	pub fn sock_read_proc(&mut self) -> Result<(),Box<dyn Error>> {
		let mut completed:i32 = self.sock.complete_read()?;
		if completed == 0 {
			return Ok(());
		}

		self.rdeidx += 1;
		self.rdeidx %= self.rdvecs.len();
		self.rdlen += 1;

		if self.insertrd > 0 {
			unsafe {
				let _ = &(*(self.evmain)).remove_event(self.rdhd);	
			}
			
		}
		self.insertrd = 0;

		loop {
			if self.rdlen == self.rdvecs.len() {
				self._write_stdout_inner()?;
			}
			let _rdptr = (&mut self.rdvecs[self.rdeidx]) as *mut u8;
			completed = self.sock.read(_rdptr,1)?;
			if completed  == 0 {
				self._write_stdout_inner()?;
				break;
			}

			self.rdlen += 1;
			self.rdeidx += 1;
			self.rdeidx %= self.rdvecs.len();
		}	

		self.rdhd = self.sock.get_read_handle();
		unsafe {
			let _ = &(*self.evmain).add_event(Arc::new(self),self.rdhd,READ_EVENT)?;
		}
		self.insertrd = 1;
		Ok(())
	}

	pub fn sock_write_proc(&mut self) -> Result<(),Box<dyn Error>> {
		let completed = self.sock.complete_write()?;
		if completed == 0 {
			return Ok(());
		}		
		self.sockwbuf = Vec::new();
		if self.insertwr > 0 {
			unsafe {
				let _ = &(*self.evmain).remove_event(self.wrhd);
			}				
		}
		self.insertwr = 0;
		return self._conti_write_sock();
	}

	pub fn connect_client(ipaddr :&str, port :u32,timemills :i32, evtmain :&mut EvtMain) -> Result<Self,Box<dyn Error>> {
		let mut retv :Self = Self {
			sock : TcpSockHandle::connect_client(ipaddr,port,"",0,false)?,
			rdvecs : Vec::with_capacity(RDBUF_SIZE),
			rdsidx : 0,
			rdeidx : 0,
			rdlen : 0,
			insertrd : 0,
			insertconn : 0,
			insertwr : 0,
			sockwbuf : Vec::new(),
			sockwbufs : Vec::new(),
			connguid : 0,
			insertconntimeout : 0,
			connhd : INVALID_EVENT_HANDLE,
			rdhd : INVALID_EVENT_HANDLE,
			wrhd : INVALID_EVENT_HANDLE,

			stdinrd : StdinRd::new()?,
			stdinvecs : Vec::with_capacity(RDBUF_SIZE),
			stdinrdsidx : 0,
			stdinrdeidx : 0,
			stdinrdlen : 0,
			insertstdinrd : 0,
			stdinrdhd : INVALID_EVENT_HANDLE,
			evmain : (evtmain as *mut EvtMain),
		};

		if retv.sock.is_connect_mode() {
			retv.connhd = retv.sock.get_connect_handle();
			evtmain.add_event(Arc::new(&mut retv),retv.connhd,WRITE_EVENT)?;
			retv.insertconn = 1;
			retv.connguid = evtmain.add_timer(Arc::new(&mut retv),timemills,false)?;
			retv.insertconntimeout = 1;
		} else {
			retv.sock_read_proc()?;
			retv.stdin_read_proc()?;
		}
		Ok(retv)
	}

	pub fn check_clear_evmain(&mut self) {
		if self.insertconntimeout == 0 && self.insertstdinrd == 0 && self.insertrd == 0 && self.insertwr == 0 && self.insertconn == 0 {
			self.evmain = std::ptr::null_mut::<EvtMain>();
		}
		return;		
	}

	pub fn close(&mut self) {
		self.close_timer_inner();
		self.close_event_inner();

		self.rdsidx = 0;
		self.rdeidx = 0;
		self.rdlen = 0;

		self.evmain = std::ptr::null_mut::<EvtMain>();

		self.stdinrdsidx = 0;
		self.stdinrdeidx = 0;
		self.stdinrdlen = 0;
	}

	pub fn connect_handle(&mut self) -> Result<(),Box<dyn Error>> {
		let completed = self.sock.complete_connect()?;
		if completed > 0 {
			if self.insertconn > 0 {
				unsafe {
					let _ = &(*self.evmain).remove_event(self.connhd);
				}
				self.insertconn = 0;
			}
			if self.insertconntimeout > 0 {
				unsafe {
					let _ = &(*self.evmain).remove_timer(self.connguid);
				}
				self.insertconntimeout = 0;
			}

			self.sock_read_proc()?;
			self.stdin_read_proc()?;
		}
		Ok(())
	}

	pub fn connect_timeout(&mut self) -> Result<(),Box<dyn Error>> {
		extargs_new_error!{EvtChatError,"connect timeout"}
	}
}


struct EvtChatServerConn {
	sock :TcpSockHandle,
	svr :*mut EvtChatServer,
	evmain :*mut EvtMain,
	wrhd :u64,
	rdhd :u64,
	inrd : i32,
	inwr : i32,
	rdbuf : Vec<u8>,
	rdsidx : usize,
	rdeidx : usize,
	rdlen : usize,
	sockwbuf : Vec<u8>,
	sockwbufs :Vec<Vec<u8>>,
}

struct EvtChatServer {
	sock :TcpSockHandle,
	evmain :*mut EvtMain,
	accsocks :Vec<EvtChatServerConn>,
	inacc : i32,
	acchd :u64,	
}

impl Drop for EvtChatServerConn {
	fn drop(&mut self) {
		self.close();
	}
}


impl EvtChatServerConn {

	fn _write_sock_inner(&mut self) -> Result<(),Box<dyn Error>> {
		let mut completed :i32;
		if self.inwr == 0 {
			loop {
				if self.sockwbuf.len() > 0 {
					let _wrptr = self.sockwbuf.as_mut_ptr() ;
					let _wrlen = self.sockwbuf.len() as u32;
					completed = self.sock.write(_wrptr,_wrlen)?;
					if completed == 0 {
						self.wrhd = self.sock.get_write_handle();
						unsafe {
							let _ = &(*self.evmain).add_event(Arc::new(self as * mut dyn EvtCall),self.wrhd,WRITE_EVENT)?;
						}
						self.inwr = 1;
						return Ok(());
					}
				}
				self.sockwbuf = Vec::new();
				if self.sockwbufs.len() == 0 {
					return Ok(());
				}

				self.sockwbuf = self.sockwbufs[0].clone();
				self.sockwbufs.remove(0);				
			}
		}
		Ok(())
	}

	fn _add_sock_write(&mut self) -> Result<(),Box<dyn Error>> {
		let mut wbuf :Vec<u8> = Vec::new();
		let mut idx :usize = 0;
		let mut curidx :usize;

		while idx < self.rdlen {
			curidx = self.rdsidx + idx;
			curidx %= self.rdbuf.len();
			wbuf.push(self.rdbuf[curidx]);
			idx += 1;
		}
		self.rdsidx = self.rdeidx;
		self.rdlen  = 0;

		if self.sockwbuf.len() == 0 {
			self.sockwbuf = wbuf;
		} else {
			self.sockwbufs.push(wbuf);
		}

		self._write_sock_inner()?;
		Ok(())
	}

	fn _read_sock_inner(&mut self) -> Result<(),Box<dyn Error>> {
		let mut completed :i32;
		loop {
			if self.rdlen == self.rdbuf.len() {
				self._add_sock_write()?;
			}

			let _rdptr = &mut self.rdbuf[self.rdeidx];
			completed = self.sock.read(_rdptr,1)?;
			if completed == 0 {
				/*to add write socket*/
				self._add_sock_write()?;
				if self.inrd == 0 {
					self.rdhd = self.sock.get_read_handle();
					unsafe {
						let _ = &(*self.evmain).add_event(Arc::new(self as *mut dyn EvtCall),self.rdhd,READ_EVENT)?;
					}
					self.inrd = 1;
				}
				return Ok(());
			}

			self.rdeidx += 1;
			self.rdeidx %= self.rdbuf.len();
			self.rdlen += 1;
		}
	}

	pub fn new(sock :TcpSockHandle,svr :*mut EvtChatServer, evmain :*mut EvtMain) -> Result<Self,Box<dyn Error>> {
		let mut retv :Self = Self {
			sock :sock,
			svr : svr,
			evmain : evmain,
			inrd : 0,
			inwr : 0,
			rdbuf : Vec::with_capacity(RDBUF_SIZE),
			rdlen : 0,
			rdeidx : 0,
			rdsidx : 0,
			sockwbuf : Vec::new(),
			sockwbufs  :Vec::new(),
			rdhd : INVALID_EVENT_HANDLE,
			wrhd : INVALID_EVENT_HANDLE,
		};

		retv._read_sock_inner()?;
		retv._write_sock_inner()?;

		Ok(retv)
	}

	pub fn write_proc(&mut self) -> Result<(),Box<dyn Error>> {
		if self.inwr > 0 {
			let completed = self.sock.complete_write()?;
			if completed == 0 {
				return Ok(());
			}

			unsafe {
				let _ = &(*self.evmain).remove_event(self.wrhd);
			}
			self.inwr = 0;
		}
		self._write_sock_inner()?;
		Ok(())
	}

	pub fn read_proc(&mut self) -> Result<(),Box<dyn Error>> {
		if self.inrd > 0 {
			let completed = self.sock.complete_read()?;
			if completed == 0 {
				return Ok(());
			}

			unsafe {
				let _ = &(*self.evmain).remove_event(self.rdhd);	
			}			
			self.inrd = 0;
		}

		self._read_sock_inner()?;
		Ok(())
	}

	fn check_clear_evmain(&mut self) {
		if self.inrd == 0 && self.inwr == 0 {
			self.evmain = std::ptr::null_mut::<EvtMain>();

			if self.svr != std::ptr::null_mut::<EvtChatServer>() {
				unsafe {
					(*self.svr).remove_client(self as *mut EvtChatServerConn);	
				}
				
				self.svr = std::ptr::null_mut::<EvtChatServer>();
			}
		}
	}

	pub fn close_event_inner(&mut self) {
		self.sock.close();
		if self.inrd > 0 {
			unsafe {
				let _ = &(*self.evmain).remove_event(self.rdhd);
			}
			self.inrd = 0;
		}

		if self.inwr > 0 {
			unsafe {
				let _ = &(*self.evmain).remove_event(self.wrhd);
			}
			self.inwr = 0;
		}

		self.check_clear_evmain();
		return;
	}

	pub fn close(&mut self) {
		self.close_event_inner();
		self.wrhd = INVALID_EVENT_HANDLE;
		self.rdhd = INVALID_EVENT_HANDLE;
		self.rdbuf = Vec::with_capacity(RDBUF_SIZE);
		self.rdsidx = 0;
		self.rdeidx = 0;
		self.rdlen = 0;

		self.sockwbuf = Vec::new();
		self.sockwbufs = Vec::new();
		return;
	}
}

impl EvtCall for EvtChatServerConn {
	fn handle(&mut self,evthd :u64, evttype :u32,_evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>> {
		let ores :Result<(),Box<dyn Error>>;
		if evthd == self.wrhd {
			ores = self.write_proc();
		} else if evthd == self.rdhd {
			ores = self.read_proc();
		} else {
			extargs_new_error!{EvtChatError,"not valid handle 0x{:x} and evttype {}",evthd,evttype}
		}
		if ores.is_err() {
			/*we close remove*/
			self.close();
		}

		Ok(())
	}

	fn close_event(&mut self,_evthd :u64, _evttype :u32, _evtmain :&mut EvtMain)  {
		self.close_event_inner();
	}
}

impl Drop for EvtChatServer {
	fn drop(&mut self) {
		self.close();
	}
}

impl EvtChatServer {

	pub fn remove_client(&mut self, chld :*mut EvtChatServerConn) {

		let mut fidx :i32 = -1;
		let mut idx :usize = 0;
		while idx < self.accsocks.len() {
			let _ptr = &mut self.accsocks[idx] as *mut EvtChatServerConn;
			if _ptr == chld {
				fidx = idx as i32;
				break;
			}
			idx += 1;
		}
		if fidx < 0 {
			return;
		}
		self.accsocks.remove(fidx as usize);
		return;
	}

	fn _inner_accept(&mut self) -> Result<(),Box<dyn Error>> {
		if self.inacc == 0 {
			loop {
				let nsock :TcpSockHandle = self.sock.accept_socket()?;
				let nconn :EvtChatServerConn = EvtChatServerConn::new(nsock,self as *mut EvtChatServer,self.evmain)?;
				self.accsocks.push(nconn);
				if self.sock.is_accept_mode() {
					break;
				}
				/*we have some thing to read*/
				let completed = self.sock.complete_accept()?;
				if completed == 0 {
					break;
				}
			}
			self.acchd = self.sock.get_accept_handle();
			unsafe {
				let _ = &(*self.evmain).add_event(Arc::new(self as *mut dyn EvtCall),self.acchd,READ_EVENT)?;
			}
			self.inacc = 1;
		}
		Ok(())
	}

	fn accept_proc(&mut self) -> Result<(),Box<dyn Error>> {
		if self.inacc > 0 {
			let completed = self.sock.complete_accept()?;
			if completed == 0 {
				return Ok(());
			}
			unsafe {
				let _ = &(*self.evmain).remove_event(self.acchd);
			}
			self.inacc = 0;
		}
		self._inner_accept()?;
		Ok(())
	}

	pub fn bind_server(ipaddr :&str, port :u32,backlog :i32, evtmain :*mut EvtMain) -> Result<Self,Box<dyn Error>> {
		let mut retv :Self = Self {
			sock : TcpSockHandle::bind_server(ipaddr,port,backlog)?,
			evmain : evtmain,
			accsocks : Vec::new(),
			inacc : 0,
			acchd : INVALID_EVENT_HANDLE,
		};
		if !retv.sock.is_accept_mode() {
			retv._inner_accept()?;
		} else {
			retv.acchd = retv.sock.get_accept_handle();
			unsafe {
				(*retv.evmain).add_event(Arc::new(&mut retv as *mut EvtChatServer as *mut dyn EvtCall),retv.acchd,READ_EVENT)?;
			}
			retv.inacc = 1;
		}
		assert!(retv.inacc > 0);
		Ok(retv)
	}

	pub fn close_event_inner(&mut self) {
		loop {
			let nsize = self.accsocks.len();
			if nsize == 0 {
				break;
			}
			self.accsocks[0].close();
		}
		if self.inacc > 0 {
			assert!(self.evmain != std::ptr::null_mut::<EvtMain>());
			unsafe {
				let _ = &(*self.evmain).remove_event(self.acchd);
			}
			self.inacc = 0;
		}

		self.evmain = std::ptr::null_mut::<EvtMain>();
		return;
	}

	pub fn close(&mut self) {
		self.close_event_inner();
		self.acchd = INVALID_EVENT_HANDLE;
	}
}

#[allow(unused_variables)]
impl EvtCall for EvtChatServer {
	fn handle(&mut self,evthd :u64, evttype :u32,evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>> {
		return self.accept_proc();
	}

	fn close_event(&mut self,evthd :u64, evttype :u32, evtmain :&mut EvtMain)  {
		self.close_event_inner();
	}
}