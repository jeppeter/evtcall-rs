
//use crate::mainloop::EvtMain;
use std::error::Error;

use crate::mainloop::{EvtMain};

pub trait EvtCall {
	fn debug_mode(&mut self,fname :&str, lineno :u32);
	fn handle(&mut self,evthd :u64, evttype :u32,evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>>;
	fn close_event(&mut self,evthd :u64, evttype :u32,evtmain :&mut EvtMain);
}

pub trait EvtTimer {
	fn timer(&mut self,timerguid :u64,evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>>;
	fn close_timer(&mut self,timerguid :u64, evtmain :&mut EvtMain);
}