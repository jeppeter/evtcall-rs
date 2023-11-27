
#[cfg(target_os = "linux")]
use libc::{clock_gettime,CLOCK_MONOTONIC_COARSE,timespec};

#[cfg(target_os = "windows")]
use winapi::um::sysinfoapi::*;

const MAX_U64_VAL :u64 = 0xffffffffffffffff;

#[cfg(target_os = "linux")]
pub (crate) fn get_cur_ticks() -> u64 {
	let mut  curtime = timespec {
		tv_sec : 0,
		tv_nsec : 0,
	};
	unsafe {clock_gettime(CLOCK_MONOTONIC_COARSE,&mut curtime);};
	let mut retmills : u64 = 0;
	retmills += (curtime.tv_sec as u64 )  * 1000;
	retmills += ((curtime.tv_nsec as u64) % 1000000000) / 1000000;
	return retmills;
}

#[cfg(target_os = "windows")]
pub (crate) fn get_cur_ticks() -> u64 {
	let retv :u64;
	unsafe {
		retv = GetTickCount64() as u64;
	}
	return retv;
}


pub (crate) fn time_left(sticks : u64,cticks :u64, leftmills :i32) -> i32 {
	let eticks = sticks + leftmills as u64;
	if cticks < eticks && cticks >= sticks {
		return (eticks - cticks) as i32;
	}

	if (MAX_U64_VAL - sticks) < (leftmills as u64) {
		if cticks > 0 && cticks < (leftmills as u64 - (MAX_U64_VAL - sticks)) {
			return ((leftmills as u64) - (MAX_U64_VAL - sticks) - cticks) as i32;
		}

		if cticks >= sticks && cticks < MAX_U64_VAL {
			return ((leftmills as u64) - (cticks - sticks)) as i32;
		}
	}
	return -1;
}