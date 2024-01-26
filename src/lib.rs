

pub mod consts;
#[macro_use]
pub mod errors;
pub (crate) mod logger;
mod timeop;

pub (crate) mod sockhdltype;
pub mod sockhdl;

//#[cfg(target_os = "windows")]
//pub (crate) mod mainloop_windows;

#[cfg(target_os = "windows")]
pub mod consts_windows;


//#[cfg(target_os = "windows")]
//pub mod sockhdl_windows;


//#[cfg(target_os = "linux")]
//pub mod sockhdl_linux;

//#[cfg(target_os = "linux")]
//pub (crate) mod mainloop_linux;

pub mod mainloop;
pub mod interface;
pub mod eventfd;
