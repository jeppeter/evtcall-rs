
use std::sync::{mpsc,Arc};
use crate::*;
use crate::consts_windows::*;
use std::error::Error;
use std::cell::RefCell;


use crate::logger::*;

evtcall_error_class!{EvtChannelError}

pub struct EvtChannelInner<T : std::marker::Send + 'static > {
	snd : mpsc::Sender<T>,
	rcv : mpsc::Receiver<T>,
	evt : i32,
	name :String,
}

impl<T : std::marker::Send + 'static > Drop for EvtChannelInner<T> {
	fn drop(&mut self) {
		self.close();
	}
}

impl<T: std::marker::Send + 'static > EvtChannelInner<T> {
	pub fn close(&mut self) {
		evtcall_log_trace!("close EvtChannelInner");
		if self.evt >= 0 {
			unsafe {
				libc::close(self.evt);
			}
			self.evt = -1;
		}
	}

	pub (crate) fn new(_maxsize :usize, s :&str) -> Result<Arc<RefCell<Self>>,Box<dyn Error>> {
		let (tx,rx) = mpsc::channel::<T>();
		let mut retv : Self = Self {
			snd : tx,
			rcv : rx,
			evt : -1,
			name :format!("{}",s),
		};

		let flags :libc::c_int = libc::EFD_NONBLOCK;
		unsafe {
			retv.evt = libc::eventfd(0,flags);
		}
		if retv.evt < 0 {
			let erri = get_errno!();
			evtcall_new_error!{EvtChannelError,"cannot eventfd {} {}",retv.name,erri}
		}

		Ok(Arc::new(RefCell::new(retv)))
	}

	pub (crate) fn put(&self,val :T) -> Result<(),Box<dyn Error>> {
		let mut reti :libc::c_int;
		let val : libc::eventfd_t = 1;
		self.snd.send(val)?;
		unsafe {
			reti = libc::eventfd_write(self.evt,val);
		}
		if reti < 0 {
			reti = get_errno!();
			evtcall_new_error!{EventFdError,"can not set event {} error {}",self.name,reti}
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
		let mut val :libc::eventfd_t = 0;
		let mut reti :libc::c_int;
		unsafe {
			let _ptr = &mut val;
			reti = libc::eventfd_read(self.evt,_ptr);
		}
		if reti < 0 {
			reti = get_errno!();
			if reti == -libc::EAGAIN || reti == -libc::EWOULDBLOCK {
				return Ok(());
			}
			evtcall_new_error!{EvtChannelError,"{} read error {}",self.name,reti}
		} 
		Ok(())
	}
}

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