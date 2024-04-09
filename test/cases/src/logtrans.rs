use extargsparse_codegen::{extargs_load_commandline,extargs_map_function};
use extargsparse_worker::namespace::{NameSpaceEx};
use extargsparse_worker::funccall::ExtArgsParseFunc;
use extargsparse_worker::parser::ExtArgsParser;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::error::Error;
use extlog::loglib::{ExtLogVar,ext_init_log};

#[extargs_map_function()]
pub fn prepare_log(parser :ExtArgsParser) -> Result<(),Box<dyn Error>> {
	let cmdline = r#"{
		"verbose|v" : "+",
		"log-files##fname[,fsize,numfiles] set write rotate files##" : [],
		"log-appends##fname[,fsize,numfiles] set append files##" : [],
		"log-nostderr##specified no stderr output##" : false
	}"#;
	extargs_load_commandline!(parser,cmdline)?;
	Ok(())	
}


pub fn init_log(ns :NameSpaceEx) -> Result<(),Box<dyn Error>> {
	let mut logvar :ExtLogVar = ExtLogVar::default();
	logvar.logverbose = ns.get_int("verbose");
	logvar.lognostderr = ns.get_bool("log_nostderr");
	logvar.logfiles = ns.get_array("log_files");
	logvar.logapps = ns.get_array("log_appends");
	return ext_init_log(&logvar);
}