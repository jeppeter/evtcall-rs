#[allow(unused_imports)]
use extargsparse_codegen::{extargs_load_commandline,ArgSet,extargs_map_function};
#[allow(unused_imports)]
use extargsparse_worker::{extargs_error_class,extargs_new_error};
#[allow(unused_imports)]
use extargsparse_worker::namespace::{NameSpaceEx};
#[allow(unused_imports)]
use extargsparse_worker::argset::{ArgSetImpl};
use extargsparse_worker::parser::{ExtArgsParser};
use extargsparse_worker::funccall::{ExtArgsParseFunc};


use std::cell::RefCell;
use std::sync::Arc;
use std::error::Error;
use std::boxed::Box;
#[allow(unused_imports)]
use regex::Regex;
#[allow(unused_imports)]
use std::any::Any;

use lazy_static::lazy_static;
use std::collections::HashMap;

#[allow(unused_imports)]
use super::{debug_trace,debug_buffer_trace,format_buffer_log,format_str_log,debug_error};
#[allow(unused_imports)]
use super::loglib::{log_get_timestamp,log_output_function,init_log};
use super::strop::{parse_u64};

use evtcall::interface::*;
use evtcall::consts::*;
use evtcall::mainloop::EvtMain;
use evtcall::sockhdl::{TcpSockHandle,init_socket,fini_socket};
use std::io::{Write};

use super::exithdl::*;

extargs_error_class!{EvtChatError}

#[cfg(target_os = "windows")]
include!("evtchat_windows.rs");



const  DEFAULT_EVCHAT_IPADDR :&str = "127.0.0.1";
const  DEFAULT_EVCHAT_LISTEN_ADDR :&str = "0.0.0.0";
const DEFAULT_EVCHAT_PORT :u32 = 4012;

fn evchatcli_handler(ns :NameSpaceEx,_optargset :Option<Arc<RefCell<dyn ArgSetImpl>>>,_ctx :Option<Arc<RefCell<dyn Any>>>) -> Result<(),Box<dyn Error>> {
	let mut evtmain :EvtMain = EvtMain::new(0)?;
	let mut ipaddr :String = format!("{}",DEFAULT_EVCHAT_IPADDR);
	let mut port :u32 = DEFAULT_EVCHAT_PORT;
	let mut evcli :EvtChatClient;
	let exithd :u64;
	let sarr :Vec<String>;
	init_log(ns.clone())?;
	sarr = ns.get_array("subnargs");
	if sarr.len() > 0 {
		port = parse_u64(&sarr[0])? as u32;
	}
	if sarr.len() > 1 {
		ipaddr = format!("{}",sarr[1]);
	}

	let _ = init_socket()?;
	exithd = init_exit_handle()?;
	evcli = EvtChatClient::connect_client(&ipaddr,port,5000,exithd,&mut evtmain)?;
	debug_trace!(" ");
	let _ = evtmain.main_loop()?;
	debug_trace!(" ");
	evcli.close();
	debug_trace!(" ");
	evtmain.close();
	debug_trace!(" ");
	fini_exit_handle();
	debug_trace!(" ");
	fini_socket();
	debug_trace!(" ");
	Ok(())
}

#[allow(unused_variables)]
fn evchatsvr_handler(ns :NameSpaceEx,_optargset :Option<Arc<RefCell<dyn ArgSetImpl>>>,_ctx :Option<Arc<RefCell<dyn Any>>>) -> Result<(),Box<dyn Error>> {
	let mut evtmain :EvtMain = EvtMain::new(0)?;
	let mut ipaddr :String = format!("{}",DEFAULT_EVCHAT_LISTEN_ADDR);
	let mut port :u32 = DEFAULT_EVCHAT_PORT;
	let mut evsvr :EvtChatServer;
	let sarr :Vec<String>;
	let exithd :u64;
	init_log(ns.clone())?;
	sarr = ns.get_array("subnargs");
	if sarr.len() > 0 {
		port = parse_u64(&sarr[0])? as u32;
	}
	if sarr.len() > 1 {
		ipaddr = format!("{}",sarr[1]);
	}

	let _ = init_socket()?;
	exithd = init_exit_handle()?;
	evsvr = EvtChatServer::bind_server(&ipaddr,port,5,exithd,&mut evtmain)?;
	evsvr.debug_mode(file!(),line!());
	let _ = evtmain.main_loop()?;	
	evsvr.close();
	evtmain.close();
	fini_exit_handle();
	fini_socket();
	Ok(())
}

#[extargs_map_function(evchatcli_handler,evchatsvr_handler)]
pub fn load_evchat_handler(parser :ExtArgsParser) -> Result<(),Box<dyn Error>> {
	let cmdline :String= format!(r#"
	{{
		"evchatsvr<evchatsvr_handler>##[port] default {}##" : {{
			"$" : "*"
		}},
		"evchatcli<evchatcli_handler>##[port] [ipaddr] default {}:{}##" : {{
			"$" : "*"
		}}
	}}
	"#,DEFAULT_EVCHAT_PORT,DEFAULT_EVCHAT_IPADDR,DEFAULT_EVCHAT_PORT);
	extargs_load_commandline!(parser,&cmdline)?;
	Ok(())
}