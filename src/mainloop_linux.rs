

use crate::interface::*;
use std::sync::Arc;
use std::error::Error;
#[allow(unused_imports)]
use std::collections::HashMap;



#[allow(dead_code)]
pub struct MainLoopLinux {
}

impl MainLoopLinux {
	pub fn new() -> Result<Self,Box<dyn Error>> {
		Ok(Self {
		})
	}

	#[allow(unused_variables)]
	pub fn add_timer(&mut self,bv :Arc<*mut dyn EvtTimer>,interval:i32,conti:bool) -> Result<u64,Box<dyn Error>> {
		unimplemented!()
	}

	#[allow(unused_variables)]
	pub fn add_event(&mut self,bv :Arc<*mut dyn EvtCall>) -> Result<(),Box<dyn Error>> {
		unimplemented!()
	}

	#[allow(unused_variables)]
	pub fn remove_timer(&mut self,guid:u64) -> Result<(),Box<dyn Error>> {
		unimplemented!()
	}

	#[allow(unused_variables)]
	pub fn remove_event(&mut self,bv :Arc<*mut dyn EvtCall>) -> Result<(),Box<dyn Error>> {
		unimplemented!()
	}

	#[allow(unused_variables)]
	pub fn main_loop(&mut self) -> Result<(),Box<dyn Error>> {
		unimplemented!()
	}

	#[allow(unused_variables)]
	pub fn break_up(&mut self) -> Result<(),Box<dyn Error>> {
		unimplemented!()
	}

	#[allow(unused_variables)]
	pub fn reset_all(&mut self) {
		unimplemented!()
	}
}

