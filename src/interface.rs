
//use crate::mainloop::EvtMain;
use std::error::Error;

use crate::mainloop::{EvtMain};

pub trait EvtCall {
	fn handle(&mut self,evthd :u64, evttype :u32,evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>>;
}

pub trait EvtTimer {
	fn timer(&mut self,timerguid :u64,evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>>;
}