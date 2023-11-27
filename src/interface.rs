
//use crate::mainloop::*;
use std::error::Error;

#[cfg(target_os = "linux")]
use crate::mainloop_linux::*;

#[cfg(target_os = "windows")]
use crate::mainloop_windows::*;


pub trait EvtCall {
	fn get_evt(&self) -> u64;
	fn get_evttype(&self) -> u32;
	fn read(&mut self,evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>>;
	fn write(&mut self,evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>>;
	fn error(&mut self,evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>>;
}

pub trait EvtTimer {
	fn timer(&mut self,evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>>;
}