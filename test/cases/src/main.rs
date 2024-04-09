#[allow(unused_imports)]
use extargsparse_codegen::{extargs_load_commandline,ArgSet,extargs_map_function};
#[allow(unused_imports)]
use extargsparse_worker::{extargs_error_class,extargs_new_error};
#[allow(unused_imports)]
use extargsparse_worker::namespace::{NameSpaceEx};
#[allow(unused_imports)]
use extargsparse_worker::options::{ExtArgsOptions};
#[allow(unused_imports)]
use extargsparse_worker::argset::{ArgSetImpl};
use extargsparse_worker::parser::{ExtArgsParser};
use extargsparse_worker::funccall::{ExtArgsParseFunc};
#[allow(unused_imports)]
use extargsparse_worker::const_value::{COMMAND_SET,SUB_COMMAND_JSON_SET,COMMAND_JSON_SET,ENVIRONMENT_SET,ENV_SUB_COMMAND_JSON_SET,ENV_COMMAND_JSON_SET,DEFAULT_SET};


#[allow(unused_imports)]
use std::cell::RefCell;
#[allow(unused_imports)]
use std::sync::Arc;
#[allow(unused_imports)]
use std::error::Error;
use std::boxed::Box;
#[allow(unused_imports)]
use regex::Regex;
#[allow(unused_imports)]
use std::any::Any;
use lazy_static::lazy_static;
use std::collections::HashMap;

mod logtrans;
mod strop;
mod fileop;
mod exithdl;
mod evttcphdl;
mod thrtst;


#[extargs_map_function()]
fn main() -> Result<(),Box<dyn Error>> {
	let parser :ExtArgsParser = ExtArgsParser::new(None,None)?;
	let commandline = r#"
	{
		"output|o" : null,
		"input|i" : null
	}
	"#;
	extargs_load_commandline!(parser,commandline)?;
	logtrans::prepare_log(parser.clone())?;
	evttcphdl::load_evchat_handler(parser.clone())?;
	thrtst::load_thread_handler(parser.clone())?;
	let ores = parser.parse_commandline_ex(None,None,None,None);
	if ores.is_err() {
		let e = ores.err().unwrap();
		eprintln!("{:?}", e);
		return Err(e);
	}
	return Ok(());
}
