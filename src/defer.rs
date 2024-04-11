

pub struct DeferCall {
	fnvecs : Vec<Box<dyn FnOnce() -> ()>>,
}

impl Drop for DeferCall {
	fn drop(&mut self) {
		self.close();
	}
}

impl DeferCall {
	pub fn new() -> Self {
		Self {
			fnvecs : Vec::new(),
		}
	}

	pub fn push_call<F :  FnOnce() -> () + 'static>(&mut self,f :F) -> usize {
		self.fnvecs.push(Box::new(f));
		return self.fnvecs.len();
	}



	pub fn close(&mut self) {
		while self.fnvecs.len() > 0 {
			let cf :Box<dyn FnOnce() -> ()> = self.fnvecs.pop().unwrap();
			cf();
		}
	}
}