
use std::sync::{mpsc,Arc};
use crate::*;
use crate::consts_windows::*;
use std::error::Error;
use std::cell::RefCell;



use winapi::um::winnt::{HANDLE,LPCWSTR};
use winapi::shared::minwindef::{BOOL,FALSE,TRUE};
use winapi::um::handleapi::{CloseHandle};
use winapi::um::errhandlingapi::{GetLastError};
use winapi::um::minwinbase::{LPSECURITY_ATTRIBUTES,SECURITY_ATTRIBUTES};
use winapi::um::synchapi::*;

use crate::logger::*;

evtcall_error_class!{EvtChannelError}

pub struct EvtChannelInner<T : std::marker::Send + 'static > {
	snd : mpsc::Sender<T>,
	rcv : mpsc::Receiver<T>,
	evt : HANDLE,
	name : String,
}

impl<T : std::marker::Send + 'static > Drop for EvtChannelInner<T> {
	fn drop(&mut self) {
		self.close();
	}
}

impl<T: std::marker::Send + 'static > EvtChannelInner<T> {
	pub fn close(&mut self) {
		evtcall_log_trace!("close EvtChannelInner");
		close_handle_safe!(self.evt,"evt");
	}

	pub (crate) fn new(_maxsize :usize,s :&str) -> Result<Arc<RefCell<Self>>,Box<dyn Error>> {
		let (tx,rx) = mpsc::channel::<T>();
		let mut retv : Self = Self {
			snd : tx,
			rcv : rx,
			evt : NULL_HANDLE_VALUE,
			name : format!("{}",s),
		};
		let note :String = format!("{} evt",retv.name);

		create_event_safe!(retv.evt,&note,EvtChannelError);

		Ok(Arc::new(RefCell::new(retv)))
	}

	pub (crate) fn put(&self,val :T) -> Result<(),Box<dyn Error>> {
		let bret :BOOL ;
		self.snd.send(val)?;
		unsafe {
			bret = SetEvent(self.evt);
		}
		if bret == FALSE {
			let erri = get_errno!();
			evtcall_new_error!{EvtChannelError,"can not SetEvent {} sndhd {}",self.name,erri}
		}
		Ok(())
	}

	pub (crate) fn get(&self) -> Result<Option<T>,Box<dyn Error>> {
		let mut retv :Option<T> = None;

		let bres = self.rcv.try_recv();
		if bres.is_err() {
			evtcall_log_warn!("receive {} error {}",self.name,bres.err().unwrap());
			return Ok(retv);
		}

		retv = Some(bres.unwrap());
		return Ok(retv);
	}

	pub (crate) fn get_evt(&self) -> u64 {
		return self.evt as u64;
	}

	pub (crate) fn reset_evt(&self) -> Result<(),Box<dyn Error>> {
		let bret :BOOL;
		if self.evt == NULL_HANDLE_VALUE {
			return Ok(());
		}
		unsafe {
			bret = ResetEvent(self.evt);
		}

		if bret == FALSE {
			let erri = get_errno!();
			evtcall_new_error!{EvtChannelError,"can not reset event {} error {}",self.name,erri}
		}
		Ok(())
	}
}

#[derive(Clone)]
pub struct EvtChannel<T : std::marker::Send + 'static > {
	inner : Arc<RefCell<EvtChannelInner<T>>>,
}

impl<T : std::marker::Send + 'static > Drop for EvtChannel<T> {
	fn drop(&mut self) {
		self.close();
	}
}

impl<T : std::marker::Send + 'static > EvtChannel<T> {
	pub fn close(&mut self) {
		evtcall_log_trace!("close EvtChannel");
	}

	pub fn new(maxsize :usize, s :&str) -> Result<Self, Box<dyn Error>> {
		let retv :Self = Self {
			inner : EvtChannelInner::new(maxsize,s)?,
		};
		Ok(retv)
	}

	pub fn put(&self,bv :T) -> Result<(),Box<dyn Error>> {
		return self.inner.borrow().put(bv);
	}

	pub fn get(&self) -> Result<Option<T>,Box<dyn Error>> {
		return self.inner.borrow().get();
	}

	pub fn get_evt(&self) -> u64 {
		return self.inner.borrow().get_evt();
	}

	pub fn reset_evt(&self)  -> Result<(),Box<dyn Error>> {
		return self.inner.borrow().reset_evt();
	}
}