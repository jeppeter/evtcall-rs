

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

macro_rules! get_errno_direct {
	() => {{
		let retv :u32 ;
		unsafe {
			retv = GetLastError() ;
		}
		retv
	}};
}

macro_rules! get_wsa_errno {
	() => {{
		let mut retv :i32 ;
		unsafe {
			retv = WSAGetLastError() as i32;
		}
		if retv != 0 {
			retv = -retv;
		} else {
			retv = -1;
		}
		retv
	}};
}

macro_rules! get_wsa_errno_direct {
	() => {{
		let retv :c_int;
		unsafe {
			retv = WSAGetLastError();
		}
		retv
	}};
}


macro_rules! set_errno {
	($val :expr) => {
		unsafe {
			SetLastError($val as DWORD);
		}
	};
}

macro_rules! close_handle_safe {
	($hdval : expr,$name :expr) => {
		let _bret :BOOL;
		let _errval :i32;
		evtcall_log_trace!("close {} 0x{:x}",$name,$hdval as u64);
		if $hdval != NULL_HANDLE_VALUE {
			unsafe {
				_bret = CloseHandle($hdval);
			}
			if _bret == FALSE {
				_errval = get_errno!();
				evtcall_log_warn!("CloseHandle {} error {}",$name,_errval);
			}
		}
		$hdval = NULL_HANDLE_VALUE;
	};
}

macro_rules! close_socket_safe {
	($sockval :expr , $name :expr) => {
		let _errval :i32;
		let _iret :c_int;
		evtcall_log_trace!("close {} sock 0x{:x}",$name,$sockval as u64);
		if $sockval != INVALID_SOCKET  {
			unsafe {
				_iret = closesocket($sockval);
			}

			if _iret == SOCKET_ERROR {
				_errval = get_errno!();
				if _errval != -WSANOTINITIALISED {
					evtcall_log_warn!("close 0x{:x} {} error {}",$sockval,$name,_errval);	
				}				
			}
		}
		$sockval = INVALID_SOCKET;
	};
}

macro_rules! new_ov {
	() => { {
		let c :OVERLAPPED = unsafe {std::mem::zeroed()};
		c
	}};
}

macro_rules! create_event_safe {
	($hd :expr,$name :expr,$errtyp :ty) => {
		let _errval :i32;
		let _pattr :LPSECURITY_ATTRIBUTES = std::ptr::null_mut::<SECURITY_ATTRIBUTES>() as LPSECURITY_ATTRIBUTES;
		let _pstr :LPCWSTR = std::ptr::null() as LPCWSTR;
		$hd = unsafe {CreateEventW(_pattr,TRUE,FALSE,_pstr)};
		if $hd == NULL_HANDLE_VALUE {
			_errval = get_errno!();
			evtcall_new_error!{$errtyp,"create {} error {}",$name,_errval}
		}
	};
}

macro_rules! cancel_io_safe {
	($val :expr,$hdval :expr, $ovval :expr , $name :expr) => {
		let _bret :BOOL;
		let _errval :DWORD;
		if $val > 0 {
			unsafe {
				_bret = CancelIoEx($hdval as HANDLE,&mut $ovval);
			}
			if _bret == FALSE {
				unsafe {
					_errval = GetLastError();
				}
				if _errval != ERROR_NOT_FOUND {
					evtcall_log_warn!("CancelIoEx {} error {}",$name,_errval);	
				}
				
			}
		}
		$val = 0;
	};
}
