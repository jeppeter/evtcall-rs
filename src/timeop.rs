

const MAX_U64_VAL :u64 = 0xffffffffffffffff;

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