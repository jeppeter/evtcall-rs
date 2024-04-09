
use winapi::um::winnt::{HANDLE,LPCWSTR};
use winapi::um::minwinbase::{LPSECURITY_ATTRIBUTES,SECURITY_ATTRIBUTES};
use winapi::um::synchapi::*;
use winapi::shared::minwindef::{TRUE,FALSE};
use winapi::um::errhandlingapi::{GetLastError};
use crate::consts_windows::*;

#[derive(Clone)]
pub (crate) struct ThreadEventInner {
	exithdl : HANDLE,
	setexithdl :HANDLE,
}

impl ThreadEventInner {
	pub (crate) fn new () -> Result<Self, Box<dyn Error>> {
		let mut retv :Self = Self {
			exithdl :  NULL_HANDLE_VALUE,
			setexithdl :  NULL_HANDLE_VALUE,
		};
		create_event_safe!(retv.exithdl, "exit handle", EvtThreadError);
		create_event_safe!(retv.setexithdl, "set exit handle", EvtThreadError);
		Ok(retv)
	}

	pub (crate) fn set_exit_event(&mut self) -> Result<(),Box<dyn Error>> {
		Ok(())
	}

	pub (crate) fn set_notice_exit_event(&mut self) -> Result<(),Box<dyn Error>> {
		Ok(())
	}

	pub (crate) fn get_exit_event(&self) -> u64 {
		return 0;
	}

	pub (crate) fn get_notice_exit_event(&self) -> u64 {
		return 0;
	}
}

