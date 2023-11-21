
use crate::interface::*;
use std::error::Error;
use std::cell::RefCell;
use std::sync::Arc;


#[allow(dead_code)]
pub struct EvtMain {
	epollfd :i32,	
}

impl EvtMain {
	pub fn new() -> Result<Self,Box<dyn Error>> {
		Ok(Self {
			epollfd : -1,
		})
	}

	pub fn add_timer(&mut self,_bv :Arc<RefCell<dyn EvtTimer>>) -> Result<(),Box<dyn Error>> {
		Ok(())
	}

	pub fn add_event(&mut self,_bv :Arc<RefCell<dyn EvtCall>>) -> Result<(),Box<dyn Error>> {
		Ok(())
	}

	pub fn remove_timer(&mut self,_bv :Arc<RefCell<dyn EvtTimer>>) -> Result<(),Box<dyn Error>> {
		Ok(())
	}

	pub fn remove_event(&mut self,_bv :Arc<RefCell<dyn EvtCall>>) -> Result<(),Box<dyn Error>> {
		Ok(())
	}

	pub fn main_loop(&mut self) -> Result<(),Box<dyn Error>> {
		Ok(())
	}

	pub fn break_up(&mut self) -> Result<(),Box<dyn Error>> {
		Ok(())
	}
}