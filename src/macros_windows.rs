

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
