
use winapi::um::winnt::{HANDLE};
use winapi::um::processenv::{GetStdHandle};
use winapi::um::winbase::{STD_INPUT_HANDLE};
use winapi::um::consoleapi::{PeekConsoleInputA,ReadConsoleInputA};
use winapi::um::wincontypes::{INPUT_RECORD,KEY_EVENT};
use winapi::um::errhandlingapi::{GetLastError};
use winapi::shared::minwindef::{DWORD,TRUE,FALSE,BOOL};
use winapi::um::handleapi::{CloseHandle};
//use backtrace::Backtrace;


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
		let mut dret :DWORD;
		let retval :i32;
		loop {
			unsafe {
				let _irptr = &mut ir as *mut INPUT_RECORD;
				let mut _dcnt :DWORD = 0;
				bret = PeekConsoleInputA(self.rd,_irptr,1,&mut _dcnt);
			}
			debug_trace!("PeekConsoleInputA {}",bret);
			if bret == FALSE {
				debug_trace!("will return 0");
				return Ok(0);
			}

			dret = 0;
			unsafe {
				let _irptr = &mut ir as *mut INPUT_RECORD;
				let _dptr = &mut dret;
				bret = ReadConsoleInputA(self.rd,_irptr,1,_dptr);
			}
			if bret == FALSE {
				retval = get_errno!();
				extargs_new_error!{EvtChatError,"can not ReadConsoleInputA error {}",retval}
			}

			//debug_trace!("dret 0x{:x} EventType 0x{:x} bKeyDown {} TRUE {}",dret,ir.EventType,unsafe{ir.Event.KeyEvent().bKeyDown},TRUE);
			//debug_buffer_trace!((&ir as *const INPUT_RECORD),std::mem::size_of::<INPUT_RECORD>(),"ir buffer");
			if dret == 1 &&  ir.EventType == KEY_EVENT &&  unsafe{ir.Event.KeyEvent().bKeyDown} == TRUE {
				//debug_trace!("ir.Event.KeyEvent().uChar.AsciiChar() {:p}",unsafe {ir.Event.KeyEvent().uChar.AsciiChar()});
				debug_trace!("AsciiChar 0x{:x}",unsafe{*ir.Event.KeyEvent().uChar.AsciiChar()});
				if unsafe{*ir.Event.KeyEvent().uChar.AsciiChar()} != 0 {
					unsafe {*rdptr = *ir.Event.KeyEvent().uChar.AsciiChar() as u8};
					return Ok(1);
				}
			}
			return Ok(0);
		}
	}

	pub fn close(&mut self) {
		let bret :BOOL;
		debug_trace!("StdinRd close {:p}",self);
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


struct EvtChatClientInner {
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

	exithd : u64,
	inexit : i32,

	evmain:*mut EvtMain,
}

#[derive(Clone)]
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
	fn handle(&mut self,evthd :u64, _evttype :u32,evtmain :&mut EvtMain,parent :EvtChatClient) -> Result<(),Box<dyn Error>> {
		debug_trace!("evthd 0x{:x} self {:p} connhd 0x{:x}",evthd,self,self.connhd);
		if evthd == self.connhd {
			self.connect_handle(parent)?;
		} else if evthd == self.wrhd {
			self.sock_write_proc(parent)?;
		} else if evthd == self.rdhd {
			self.sock_read_proc(parent)?;
		} else if evthd == self.stdinrd.get_handle() {
			self.stdin_read_proc(parent)?;
		} else if evthd == self.exithd {
			debug_trace!("break_up");
			return evtmain.break_up();
		} else {
			extargs_new_error!{EvtChatError,"not recognize evthd 0x{:x}",evthd}
		}
		debug_trace!("exit handle {:p}",self);
		Ok(())
	}	

	fn debug_mode(&mut self,_fname :&str, _lineno :u32,_parent :EvtChatClient) {
		debug_trace!("debugmode local {} remote {}",self.sock.get_self_format(),self.sock.get_peer_format());
		return;
	}


	fn close_event(&mut self,_evthd :u64, _evttype :u32, _evtmain :&mut EvtMain,_parent :EvtChatClient)  {
		debug_trace!("self {:p}",self);
		//let bt = Backtrace::new();
		//debug_trace!("{:?}",bt);
		self.close_event_inner();
		return;
	}
}

impl EvtChatClientInner {
	fn timer(&mut self,_timerguid :u64,_evtmain :&mut EvtMain,parent :EvtChatClient) -> Result<(),Box<dyn Error>> {
		return self.connect_timeout(parent);
	}

	fn close_timer(&mut self, _guid :u64, _evtmain :&mut EvtMain, _parent :EvtChatClient) {
		debug_trace!("self {:p}",self);
		self.close_timer_inner();
		return;
	}
}


impl EvtChatClientInner {

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

		if self.inexit > 0 {
			unsafe {
				let _ = &(*self.evmain).remove_event(self.exithd);
			}
			self.inexit = 0;
		}

		self.check_clear_evmain();
		return;
	}
	fn close_timer_inner(&mut self) {
		//let bt = Backtrace::new();
		debug_trace!("insertconntimeout {} self {:p}",self.insertconntimeout,self);
		//debug_trace!("{:?}",bt);
		if self.insertconntimeout > 0 {
			unsafe {
				let _ = &(*(self.evmain)).remove_timer(self.connguid);
			}
			debug_trace!("remove conntimer 0x{:x}",self.connguid);
			self.insertconntimeout = 0;
		}
		self.check_clear_evmain();
		return;		
	}

	fn _conti_write_sock(&mut self,parent :EvtChatClient) -> Result<(),Box<dyn Error>> {
		let mut completed :i32 = 0;
		if self.insertwr == 0 {
			loop {
				if self.sockwbuf.len() > 0 {
					let completed = self.sock.write(self.sockwbuf.as_mut_ptr(),self.sockwbuf.len() as u32)?;
					if completed > 0 {
						self.sockwbuf = Vec::new();
					} else {
						self.wrhd = self.sock.get_write_handle();
						unsafe {
							let _ = &(*self.evmain).add_event(Arc::new(RefCell::new(parent.clone())),self.wrhd,WRITE_EVENT)?;
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

	fn _sock_write_inner(&mut self,parent :EvtChatClient) -> Result<(),Box<dyn Error>> {
		let mut wbuf :Vec<u8> = Vec::new();
		let mut idx :usize =0;
		let mut curidx :usize;
		let completed :i32;
		while idx < self.stdinrdlen {
			curidx = self.stdinrdsidx + idx;
			curidx %= self.stdinvecs.capacity();
			wbuf.push(self.stdinvecs[curidx]);
			idx += 1;
		}
		self.stdinrdsidx = self.stdinrdeidx;
		self.stdinrdlen = 0;
		debug_buffer_trace!(wbuf.as_ptr(),wbuf.len(),"wbuf ");

		if self.sockwbuf.len() == 0 {
			self.sockwbuf = wbuf.clone();
			completed = self.sock.write(self.sockwbuf.as_ptr() as *mut u8,self.sockwbuf.len() as u32)?;
			debug_trace!("write completed {}",completed);
			if completed > 0 {
				if self.insertwr > 0 {
					assert!(self.wrhd != INVALID_EVENT_HANDLE);
					unsafe {
						let _ = &(*self.evmain).remove_event(self.wrhd);
					}
					self.insertwr = 0;
				}
				self.sockwbuf = Vec::new();
				debug_trace!("write over");
			} else {
				if self.insertwr == 0 {
					self.wrhd = self.sock.get_write_handle();
					unsafe {
						let _ = &(*self.evmain).add_event(Arc::new(RefCell::new(parent.clone())),self.wrhd,WRITE_EVENT)?;
					}
					self.insertwr = 1;
				}
			}
		} else {
			self.sockwbufs.push(wbuf);
		}
		self._conti_write_sock(parent.clone())?;
		Ok(())

	}

	pub fn stdin_read_proc(&mut self,parent :EvtChatClient) -> Result<(),Box<dyn Error>> {
		let mut cnt :usize = 0;
		let maxcnt :usize = 1;
		while cnt < maxcnt {
			cnt += 1;
			if self.stdinrdlen >= (self.stdinvecs.capacity() - 2) {
				self._sock_write_inner(parent.clone())?;
			}

			let _rptr = (&mut self.stdinvecs[self.stdinrdeidx]) as *mut u8;
			let completed = self.stdinrd.read(_rptr)?;
			if completed > 0 {
				let lastidx :usize = self.stdinrdeidx;
				self.stdinrdeidx += 1;
				self.stdinrdeidx %= self.stdinvecs.capacity();
				self.stdinrdlen += 1;

				if self.stdinvecs[lastidx] == '\r' as u8 {
					self.stdinvecs[self.stdinrdeidx] = '\n' as u8;
					self.stdinrdeidx += 1;
					self.stdinrdeidx %= self.stdinvecs.capacity();
					self.stdinrdlen += 1;					
				}
				debug_trace!("read stdinrdlen 0x{:x}",self.stdinrdlen);

			} else {
				break;
			}
		}
		if self.insertstdinrd == 0 {
			self.stdinrdhd = self.stdinrd.get_handle();
			unsafe {
				let _ = &(*self.evmain).add_event(Arc::new(RefCell::new(parent.clone())),self.stdinrdhd,READ_EVENT)?;
			}
			self.insertstdinrd = 1;
		}
		self._sock_write_inner(parent.clone())?;
		return Ok(());

	}

	fn _write_stdout_inner(&mut self) -> Result<(),Box<dyn Error>> {
		let mut rdvecs :Vec<u8> = Vec::new();
		let mut idx :usize = 0;
		let mut curidx :usize;
		while idx < self.rdlen {
			curidx = self.rdsidx + idx;
			curidx %= self.rdvecs.capacity();
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

	fn _read_sock_inner(&mut self,parent :EvtChatClient) -> Result<(),Box<dyn Error>> {
		loop {
			if self.rdlen == self.rdvecs.capacity() {
				self._write_stdout_inner()?;
			}
			debug_trace!("rdeidx {} self {:p} sock {:p}",self.rdeidx,self,&(self.sock));
			let _rdptr = (&mut self.rdvecs[self.rdeidx]) as *mut u8;
			debug_trace!("_rdptr 0x{:p} self.rdvecs 0x{:p}",_rdptr, self.rdvecs.as_ptr());
			let completed = (&mut self.sock).read(_rdptr,1)?;
			debug_trace!("completed {} self {:p}",completed,self);
			if completed  == 0 {
				self._write_stdout_inner()?;
				break;
			}

			self.rdlen += 1;
			self.rdeidx += 1;
			self.rdeidx %= self.rdvecs.capacity();
			debug_trace!("rdlen 0x{:x}",self.rdlen);
		}	

		debug_trace!("_read_sock_inner over self {:p}",self);
		if self.insertrd == 0 {
			self.rdhd = self.sock.get_read_handle();
			unsafe {
				let _ = &(*self.evmain).add_event(Arc::new(RefCell::new(parent.clone())),self.rdhd,READ_EVENT)?;
			}
			self.insertrd = 1;			
		}
		Ok(())
	}

	pub fn sock_read_proc(&mut self,parent :EvtChatClient) -> Result<(),Box<dyn Error>> {
		let completed:i32 = self.sock.complete_read()?;
		if completed == 0 {
			return Ok(());
		}

		self.rdeidx += 1;
		self.rdeidx %= self.rdvecs.capacity();
		self.rdlen += 1;

		if self.insertrd > 0 {
			unsafe {
				let _ = &(*(self.evmain)).remove_event(self.rdhd);	
			}			
		}
		self.insertrd = 0;

		return self._read_sock_inner(parent);
	}

	pub fn sock_write_proc(&mut self,parent :EvtChatClient) -> Result<(),Box<dyn Error>> {
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
		return self._conti_write_sock(parent);
	}

	pub fn connect_client_after(&mut self,timemills : i32,parent :EvtChatClient) -> Result<(),Box<dyn Error>> {
		unsafe {
			self.rdvecs.set_len(RDBUF_SIZE);
			self.stdinvecs.set_len(RDBUF_SIZE);
		}

		if self.sock.is_connect_mode() {
			debug_trace!("in connect mode");
			self.connhd = self.sock.get_connect_handle();
			unsafe {
				(*self.evmain).add_event(Arc::new(RefCell::new(parent.clone())),self.connhd,WRITE_EVENT)?;
			}
			
			self.insertconn = 1;
			unsafe {
				self.connguid = (*self.evmain).add_timer(Arc::new(RefCell::new(parent.clone())),timemills,false)?;	
			}
			
			self.insertconntimeout = 1;
			debug_trace!("connguid 0x{:x} connhd 0x{:x} timemills {}",self.connguid,self.connhd,timemills);
		} else {
			debug_trace!("will read mode");
			self.sock_read_proc(parent.clone())?;
			self.stdin_read_proc(parent.clone())?;
		}

		unsafe {
			(*self.evmain).add_event(Arc::new(RefCell::new(parent.clone())),self.exithd,READ_EVENT)?;
		}		
		self.inexit = 1;
		debug_trace!("insert exithd 0x{:x} {:p}",self.exithd,self);
		Ok(())
	}

	pub fn connect_client(ipaddr :&str, port :u32,exithd :u64, evtmain :&mut EvtMain) -> Result<Arc<RefCell<Self>>,Box<dyn Error>> {
		let iretv :Self = Self {
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
			exithd : exithd,
			inexit : 0,
			evmain : (evtmain as *mut EvtMain),
		};

		let retv :Arc<RefCell<EvtChatClientInner>> = Arc::new(RefCell::new(iretv));
		Ok(retv)
	}

	pub fn check_clear_evmain(&mut self) {
		if self.insertconntimeout == 0 && self.insertstdinrd == 0 && self.insertrd == 0 && self.insertwr == 0 && self.insertconn == 0 {
			self.evmain = std::ptr::null_mut::<EvtMain>();
		}
		return;		
	}

	pub fn close(&mut self) {
		debug_trace!("EvtChatClientInner close {:p}",self);
		self.close_timer_inner();
		self.close_event_inner();


		self.sock.close();
		self.connhd = INVALID_EVENT_HANDLE;
		self.rdhd = INVALID_EVENT_HANDLE;
		self.wrhd = INVALID_EVENT_HANDLE;


		self.rdvecs = Vec::new();
		self.rdsidx = 0;
		self.rdeidx = 0;
		self.rdlen = 0;

		assert!(self.insertconn == 0);
		assert!(self.insertrd == 0);
		assert!(self.insertwr == 0);

		self.sockwbuf = Vec::new();
		self.sockwbufs = Vec::new();

		self.connguid = 0;
		assert!(self.insertconntimeout == 0);

		self.stdinrd.close();
		self.stdinrdhd = INVALID_EVENT_HANDLE;

		self.stdinvecs = Vec::new();
		self.stdinrdsidx = 0;
		self.stdinrdeidx = 0;
		self.stdinrdlen = 0;
		assert!(self.insertstdinrd == 0);

		self.exithd = INVALID_EVENT_HANDLE;
		assert!(self.inexit == 0);
		assert!(self.evmain == std::ptr::null_mut::<EvtMain>());
	}

	pub fn connect_handle(&mut self,parent :EvtChatClient) -> Result<(),Box<dyn Error>> {
		debug_trace!("will handle connect");
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
				debug_trace!("remove conn timer 0x{:x} insertconntimeout {} self {:p}",self.connguid,self.insertconntimeout,self);
			}

			debug_trace!("connect {} => {}",self.sock.get_self_format(),self.sock.get_peer_format());

			self._read_sock_inner(parent.clone())?;
			self.stdin_read_proc(parent.clone())?;
		}
		debug_trace!("exit connect_handle {:p}",self);
		Ok(())
	}

	pub fn connect_timeout(&mut self,_parent :EvtChatClient) -> Result<(),Box<dyn Error>> {
		extargs_new_error!{EvtChatError,"connect timeout"}
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
		return self.inner.borrow_mut().close_event(evthd,evttype,evtmain,p);
	}
}

impl EvtTimer for EvtChatClient {
	fn timer(&mut self,_timerguid :u64,_evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>> {
		let p :EvtChatClient = self.clone();
		return self.inner.borrow_mut().timer(_timerguid,_evtmain,p);
	}

	fn close_timer(&mut self, _guid :u64, _evtmain :&mut EvtMain) {
		let p :EvtChatClient = self.clone();
		return self.inner.borrow_mut().close_timer(_guid,_evtmain,p);
	}
}


struct EvtChatServerConnInner {
	sock :TcpSockHandle,
	svr :*mut EvtChatServerInner,
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

#[derive(Clone)]
struct EvtChatServerConn {
	inner :Arc<RefCell<EvtChatServerConnInner>>,
	socknum : u64,
}

struct EvtChatServerInner {
	sock :TcpSockHandle,
	evmain :*mut EvtMain,
	accsocks :Vec<EvtChatServerConn>,
	inacc : i32,
	acchd :u64,	
	exithd :u64,
	inexit : i32,
}

#[derive(Clone)]
struct EvtChatServer {
	inner :Arc<RefCell<EvtChatServerInner>>,
}

impl Drop for EvtChatServerConnInner {
	fn drop(&mut self) {
		self.close();
	}
}

impl Drop for EvtChatServerConn {
	fn drop(&mut self) {
		self.close();
	}
}


impl EvtChatServerConnInner {

	fn _write_sock_inner(&mut self,parent :EvtChatServerConn) -> Result<(),Box<dyn Error>> {
		let mut completed :i32;
		if self.inwr == 0 {
			loop {
				if self.sockwbuf.len() > 0 {
					let _wrptr = self.sockwbuf.as_mut_ptr() ;
					let _wrlen = self.sockwbuf.len() as u32;
					debug_buffer_trace!(_wrptr,_wrlen,"write buffer");
					completed = self.sock.write(_wrptr,_wrlen)?;
					if completed == 0 {
						self.wrhd = self.sock.get_write_handle();
						unsafe {
							let _ = &(*self.evmain).add_event(Arc::new(RefCell::new(parent.clone())),self.wrhd,WRITE_EVENT)?;
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

	fn _add_sock_write(&mut self,parent :EvtChatServerConn) -> Result<(),Box<dyn Error>> {
		let mut wbuf :Vec<u8> = Vec::new();
		let mut idx :usize = 0;
		let mut curidx :usize;

		while idx < self.rdlen {
			curidx = self.rdsidx + idx;
			curidx %= self.rdbuf.capacity();
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

		self._write_sock_inner(parent)?;
		Ok(())
	}

	fn _read_sock_inner(&mut self,parent :EvtChatServerConn) -> Result<(),Box<dyn Error>> {
		let mut completed :i32;
		loop {
			debug_trace!("self {:p}",self);
			if self.rdlen == self.rdbuf.len() {
				self._add_sock_write(parent.clone())?;
			}

			debug_trace!("self {:p}",self);
			let _rdptr = &mut self.rdbuf[self.rdeidx];
			completed = self.sock.read(_rdptr,1)?;
			debug_trace!("read completed {}",completed);
			if completed == 0 {
				/*to add write socket*/
				self._add_sock_write(parent.clone())?;
				if self.inrd == 0 {
					self.rdhd = self.sock.get_read_handle();
					unsafe {
						let _ = &(*self.evmain).add_event(Arc::new(RefCell::new(parent.clone())),self.rdhd,READ_EVENT)?;
					}
					self.inrd = 1;
				}
				return Ok(());
			}

			self.rdeidx += 1;
			self.rdeidx %= self.rdbuf.capacity();
			self.rdlen += 1;
			debug_trace!("rdlen 0x{:x}",self.rdlen);
		}
	}

	pub fn new_after(&mut self,parent:EvtChatServerConn) -> Result<(),Box<dyn Error>> {
		unsafe {
			self.rdbuf.set_len(RDBUF_SIZE);
		}

		self._read_sock_inner(parent.clone())?;
		self._write_sock_inner(parent.clone())?;
		Ok(())
	}

	pub fn new(sock :TcpSockHandle,svr :*mut EvtChatServerInner, evmain :*mut EvtMain) -> Result<Arc<RefCell<Self>>,Box<dyn Error>> {
		let  iretv :Self = Self {
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

		let retv = Arc::new(RefCell::new(iretv));
		Ok(retv)
	}

	pub fn write_proc(&mut self,parent :EvtChatServerConn) -> Result<(),Box<dyn Error>> {
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
		self._write_sock_inner(parent)?;
		Ok(())
	}

	pub fn read_proc(&mut self,parent :EvtChatServerConn) -> Result<(),Box<dyn Error>> {
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
		debug_trace!(" ");
		self._read_sock_inner(parent)?;
		Ok(())
	}

	fn check_clear_evmain(&mut self) {
		if self.inrd == 0 && self.inwr == 0 {
			self.evmain = std::ptr::null_mut::<EvtMain>();

			if self.svr != std::ptr::null_mut::<EvtChatServerInner>() {
				unsafe {
					(*self.svr).remove_client(self.sock.get_sock_real());	
				}
				
				self.svr = std::ptr::null_mut::<EvtChatServerInner>();
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
		debug_trace!("EvtChatServerConnInner close {:p}",self);
		self.close_event_inner();
		assert!(self.svr == std::ptr::null_mut::<EvtChatServerInner>());
		assert!(self.evmain == std::ptr::null_mut::<EvtMain>());
		self.wrhd = INVALID_EVENT_HANDLE;
		self.rdhd = INVALID_EVENT_HANDLE;
		assert!(self.inrd == 0);
		assert!(self.inwr == 0);

		self.rdbuf = Vec::new();
		self.rdsidx = 0;
		self.rdeidx = 0;
		self.rdlen = 0;

		self.sockwbuf = Vec::new();
		self.sockwbufs = Vec::new();
		return;
	}

	pub fn get_sock_real(&self) -> u64 {
		return self.sock.get_sock_real();
	}
}

impl EvtChatServerConnInner {
	fn handle(&mut self,evthd :u64, evttype :u32,_evtmain :&mut EvtMain,parent :EvtChatServerConn) -> Result<(),Box<dyn Error>> {
		let ores :Result<(),Box<dyn Error>>;
		if evthd == self.wrhd {
			ores = self.write_proc(parent);
		} else if evthd == self.rdhd {
			ores = self.read_proc(parent);
		} else {
			extargs_new_error!{EvtChatError,"not valid handle 0x{:x} and evttype {}",evthd,evttype}
		}
		if ores.is_err() {
			/*we close remove*/
			self.close();
		}

		Ok(())
	}

	fn debug_mode(&mut self,_fname :&str, _lineno :u32,_parent :EvtChatServerConn) {
		return;
	}


	fn close_event(&mut self,_evthd :u64, _evttype :u32, _evtmain :&mut EvtMain,_parent :EvtChatServerConn)  {
		self.close_event_inner();
	}
}

impl EvtChatServerConn {
	pub fn new(sock :TcpSockHandle,svr :*mut EvtChatServerInner, evmain :*mut EvtMain) -> Result<Self,Box<dyn Error>> {
		let ninner = EvtChatServerConnInner::new(sock,svr,evmain)?;
		let mut retv :Self = Self {
			inner : ninner,
			socknum : 0,
		};
		let p = retv.clone();
		retv.inner.borrow_mut().new_after(p)?;
		retv.socknum = retv.inner.borrow().get_sock_real();
		Ok(retv)
	}

	pub fn get_sock_real(&self) -> u64 {
		return self.socknum;
	}

	pub fn close(&mut self) {
		debug_trace!("EvtChatServerConn close {:p}",self);
		return;
	}
	pub fn get_self_format(&self) ->String {
		return self.inner.borrow().sock.get_self_format();
	}

	pub fn get_peer_format(&self) -> String {
		return self.inner.borrow().sock.get_peer_format();
	}
}

impl EvtCall for EvtChatServerConn {
	fn handle(&mut self,evthd :u64, evttype :u32,_evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>> {
		let p = self.clone();
		return self.inner.borrow_mut().handle(evthd,evttype,_evtmain,p);
	}

	fn debug_mode(&mut self,_fname :&str, _lineno :u32) {
		let p = self.clone();
		return self.inner.borrow_mut().debug_mode(_fname,_lineno,p);
	}


	fn close_event(&mut self,_evthd :u64, _evttype :u32, _evtmain :&mut EvtMain)  {
		let p = self.clone();
		return self.inner.borrow_mut().close_event(_evthd,_evttype,_evtmain,p);
	}

}


impl Drop for EvtChatServerInner {
	fn drop(&mut self) {
		self.close();
	}
}

impl Drop for EvtChatServer {
	fn drop(&mut self) {
		self.close();
	}
}

impl EvtChatServerInner {

	pub fn remove_client(&mut self, guid :u64) {
		let mut fidx :i32 = -1;
		let mut idx :usize = 0;
		while idx < self.accsocks.len() {			 
			if self.accsocks[idx].get_sock_real() == guid {
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

	fn _inner_accept(&mut self,parent :EvtChatServer) -> Result<(),Box<dyn Error>> {
		debug_trace!("self {:p}",self);
		if self.inacc == 0 {
			loop {
				debug_trace!(" ");
				let nsock :TcpSockHandle = self.sock.accept_socket()?;
				let nconn :EvtChatServerConn = EvtChatServerConn::new(nsock,self as *mut EvtChatServerInner,self.evmain)?;
				debug_trace!("get {} => {} connect", nconn.get_peer_format(),nconn.get_self_format());
				self.accsocks.push(nconn);
				if self.sock.is_accept_mode() {
					debug_trace!(" ");
					break;
				}
				/*we have some thing to read*/
				let completed = self.sock.complete_accept()?;
				if completed == 0 {
					break;
				}
			}
			self.acchd = self.sock.get_accept_handle();
			if self.acchd != INVALID_EVENT_HANDLE && self.acchd != 0 {
				debug_trace!(" will accept");
				unsafe {
					let _ = &(*self.evmain).add_event(Arc::new(RefCell::new(parent.clone())),self.acchd,READ_EVENT)?;
				}
				self.inacc = 1;				
			}
		}
		Ok(())
	}

	fn accept_proc(&mut self,parent:EvtChatServer) -> Result<(),Box<dyn Error>> {
		if self.inacc > 0 {
			debug_trace!(" ");
			let completed = self.sock.complete_accept()?;
			if completed == 0 {
				debug_trace!(" ");
				return Ok(());
			}
			unsafe {
				let _ = &(*self.evmain).remove_event(self.acchd);
			}
			self.inacc = 0;
		}
		debug_trace!(" ");
		self._inner_accept(parent)?;
		Ok(())
	}

	pub fn bind_server_after(&mut self,parent :EvtChatServer) -> Result<(),Box<dyn Error>> {
		if !self.sock.is_accept_mode() {
			self._inner_accept(parent.clone())?;
		} else {
			self.acchd = self.sock.get_accept_handle();
			debug_trace!("add accept 0x{:x} retv {:p}",self.acchd,self);
			unsafe {
				(*self.evmain).add_event(Arc::new(RefCell::new(parent.clone())),self.acchd,READ_EVENT)?;
			}
			self.inacc = 1;
		}

		debug_trace!("listen on {} retv {:p}",self.sock.get_self_format(),self);

		unsafe {
			let _ = &(*self.evmain).add_event(Arc::new(RefCell::new(parent.clone())),self.exithd,READ_EVENT)?;
		}
		self.inexit = 1;
		assert!(self.inacc > 0);
		self.debug_mode(file!(),line!(),parent.clone());
		Ok(())
	}

	pub fn bind_server(ipaddr :&str, port :u32,backlog :i32,exithd :u64, evtmain :*mut EvtMain) -> Result<Arc<RefCell<Self>>,Box<dyn Error>> {
		let iretv :Self = Self {
			sock : TcpSockHandle::bind_server(ipaddr,port,backlog)?,
			evmain : evtmain,
			accsocks : Vec::new(),
			inacc : 0,
			acchd : INVALID_EVENT_HANDLE,
			exithd : exithd,
			inexit : 0,
		};
		let retv = Arc::new(RefCell::new(iretv));
		Ok(retv)
	}

	pub fn close_event_inner(&mut self) {
		loop {
			let nsize = self.accsocks.len();
			if nsize == 0 {
				break;
			}
			self.accsocks[0].close();
			let csize = self.accsocks.len();
			if csize == nsize {
				/*it not remove ,so do remove*/
				self.accsocks.remove(0);
			}
		}

		if self.inexit > 0 {
			assert!(self.evmain != std::ptr::null_mut::<EvtMain>());
			unsafe {
				let _ = &(*self.evmain).remove_event(self.exithd);
			}
			self.inexit = 0;
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
		debug_trace!("EvtChatServerInner close {:p}",self);
		self.close_event_inner();
		assert!(self.evmain == std::ptr::null_mut::<EvtMain>());
		assert!(self.accsocks.len() == 0);
		assert!(self.inacc == 0);		
		self.acchd = INVALID_EVENT_HANDLE;
		self.exithd = INVALID_EVENT_HANDLE;
		assert!(self.inexit == 0);
	}
}

impl  EvtChatServerInner {
	fn handle(&mut self,evthd :u64, _evttype :u32,evtmain :&mut EvtMain,parent :EvtChatServer) -> Result<(),Box<dyn Error>> {
		debug_trace!("self {:p}",self);
		if evthd == self.exithd {
			evtmain.break_up()?;
		} else if evthd == self.acchd {
			self.accept_proc(parent)?;
		} else {
			extargs_new_error!{EvtChatError,"not support evthd 0x{:x}",evthd}
		}
		//debug_trace!(" ");
		Ok(())
		
	}

	fn debug_mode(&mut self,fname :&str, lineno :u32,_parent :EvtChatServer) {
		debug_trace!("{}:{}",fname,lineno);
		debug_trace!("[{}:{}]debugmode local {} remote {}",fname,lineno,self.sock.get_self_format(),self.sock.get_peer_format());
		return;
	}

	fn close_event(&mut self,_evthd :u64, _evttype :u32, _evtmain :&mut EvtMain,_parent :EvtChatServer)  {
		self.close_event_inner();
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
