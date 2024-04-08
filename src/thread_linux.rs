

pub (crate) ThreadEvt<F> 
where 
	F : FnOnce() -> (),
	F : Send + 'static ,
	F : Sync + 'static , {
	exitfd : i32,
	setexit : i32,
	callfn : F,
}

impl ThreadEvt <F> {
	pub (crate) fn new(bstart :bool, callfn :F) -> Result<Self,Box<dyn Error>> {
		let retv :Self = Self {
			exitfd : -1,
			setexit : -1,
			callfn : callfn,
		};
		Ok(retv)
	}
}