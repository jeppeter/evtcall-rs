
use winapi::um::winnt::{HANDLE,LPCWSTR};
use winapi::um::minwinbase::{LPSECURITY_ATTRIBUTES,SECURITY_ATTRIBUTES};
use winapi::um::synchapi::*;
use winapi::shared::minwindef::{TRUE,FALSE};
use winapi::um::errhandlingapi::{GetLastError};
use crate::consts_windows::*;

#[derive(Clone)]
pub (crate) struct ThreadEvent {
	exithdl : HANDLE,
	setexithdl :HANDLE,
}

impl ThreadEvent {
	pub (crate) fn new () -> Result<Self, Box<dyn Error>> {
		let mut retv :Self = Self {
			exithdl :  NULL_HANDLE_VALUE,
			setexithdl :  NULL_HANDLE_VALUE,
		};
		create_event_safe!(retv.exithdl, "exit handle", EvtThreadError);
		create_event_safe!(retv.setexithdl, "set exit handle", EvtThreadError);
		Ok(retv)
	}

	pub (crate) fn set_exited(&mut self) -> Result<(),Box<dyn Error>> {
		Ok(())
	}

}