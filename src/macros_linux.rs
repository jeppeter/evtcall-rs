
macro_rules! get_errno {
	() => {{
		let mut _retv :i32;
		unsafe {
			_retv = (*libc::__errno_location())  as i32;
		}
		if _retv > 0 {
			_retv = -_retv;
		} else if _retv == 0 {
			_retv = -1;
		}
		_retv
	}};
}

macro_rules! close_sock_safe {
	($sock :expr,$name :expr) => {
		if $sock >= 0 {
			let mut _reti :libc::c_int;
			unsafe {
				_reti = libc::close($sock);
			}
			if _reti < 0 {
				_reti = get_errno!();
				evtcall_log_error!("close {} error {}",$name,_reti);
			}
		}
		$sock = DEFAULT_SOCKET;
	};
}
