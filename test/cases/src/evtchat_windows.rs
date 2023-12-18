


struct EvtChatClient {

}

impl EvtCall for EvtChatClient {
	fn handle(&mut self,evthd :u64, evttype :u32,evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>> {
		Ok(())
	}	
}

impl EvtTimer for EvtChatClient {
	fn timer(&mut self,timerguid :u64,evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>> {
		Ok(())
	}
}

struct EvtChatServerConn {
	sock :TcpSockHandle,
	svr :*mut EvtChatServer,
}

struct EvtChatServer {
	sock :TcpSockHandle,
}

impl EvtCall for EvtChatServer {
	fn handle(&mut self,evthd :u64, evttype :u32,evtmain :&mut EvtMain) -> Result<(),Box<dyn Error>> {
		if evthd == self.sock.get_accept
		Ok(())
	}
}