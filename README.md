# evtcall
> rust event driven framework 

### Release History
* Apr 9th 2024 to release 0.1.6 for thread of EvtThread
* Feb 18th 2024 to release 0.1.4 for channel of EvtChannel
* Jan 29th 2024 to release 0.1.2 for eventfd set and is
* Jan 18th 2024 to release 0.1.0 for the first version


### to notice
```rust
struct ThreadData {
	pub val : i32,
	pub vals :String,
}

impl ThreadData {
	fn new(val :i32, vs :&str) -> Self {
		Self {
			val : val,
			vals : format!("{}",vs),
		}
	}
}

fn thread_call_new(threvt :ThreadEvent , mills : u32) -> ThreadData {
	let now = Instant::now();
	let wmills :u128 = mills as u128;
	let mut bnotified : bool =false;
	let noteevt : u64 = threvt.get_notice_exit_event();
	let mut cnt : i32 = 0;
	loop {
		let curmills = now.elapsed().as_millis();
		if curmills > wmills {
			break;
		}

		if !bnotified {
			bnotified = wait_event_fd_timeout(noteevt,10);			
		} else {
			std::thread::sleep(std::time::Duration::from_millis(10));
		}

		cnt += 1;

		if (cnt % 100) == 0 {
			debug_trace!("[{}]bnotified [{}]",cnt,bnotified);
		}
	}

	return ThreadData::new(20,"hello");
}

fn evtthr_handler(ns :NameSpaceEx,_optargset :Option<Arc<RefCell<dyn ArgSetImpl>>>,_ctx :Option<Arc<RefCell<dyn Any>>>) -> Result<(),Box<dyn Error>> {	
	let mut parentmills : u32 = 1000;
	let mut childmills : u32 = 900;
	let sarr :Vec<String>;
	let  mut evt :ThreadEvent = ThreadEvent::new().unwrap();
	let mut notwait :bool = false;

	init_log(ns.clone())?;
	sarr = ns.get_array("subnargs");
	if sarr.len() > 0 {
		parentmills = parse_u64(&sarr[0])? as u32;
	}

	if sarr.len() > 1 {
		childmills = parse_u64(&sarr[1])? as u32;
	}

	if sarr.len() > 2 {
		notwait = true;
	}

	let mut thr :EvtThread<ThreadData> = EvtThread::new(evt.clone())?;
	let oevt = evt.clone();
	thr.start(move || {
		return thread_call_new(oevt,childmills);
	})?;
	let chldevt :u64 = evt.get_exit_event();
	let now :Instant = Instant::now();
	let wmills : u128 = parentmills as u128;
	let mut bval : bool = false;
	let mut cnt : i32 = 0;
	loop {
		let curmills = now.elapsed().as_millis();
		if curmills > wmills {
			break;
		}
		if !bval {
			bval = wait_event_fd_timeout(chldevt,10);			
		} else {
			std::thread::sleep(std::time::Duration::from_millis(10));
		}

		cnt += 1;

		if (cnt % 100) == 0 {
			debug_trace!("[{}]bval [{}]",cnt,bval);
		}
	}

	let _ = evt.set_notice_exit_event();

	if !notwait {
		loop {
			if thr.is_exited() {
				break;
			}
			if !bval {
				bval = wait_event_fd_timeout(chldevt,10);			
			} else {
				std::thread::sleep(std::time::Duration::from_millis(10));
			}
			cnt += 1;

			if (cnt % 100) == 0 {
				debug_trace!("[{}]not exited",cnt);
			}
		}

		loop {
			let cdata :Option<ThreadData> = thr.get_return();
			if cdata.is_some() {
				let cptr :ThreadData = cdata.unwrap();
				debug_trace!("cdata.val {} cdata.vals {}",cptr.val,cptr.vals);
				break;
			}
			std::thread::sleep(std::time::Duration::from_millis(10));
			debug_trace!("will try again get_return");
		}		
	}
	Ok(())
}
```
notice that the notwait must not call ,this will call no free EvtThread after