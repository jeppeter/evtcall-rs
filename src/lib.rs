

pub mod consts;
#[macro_use]
pub mod errors;
mod timeop;

#[cfg(target_os = "windows")]
pub (crate) mod mainloop_windows;

#[cfg(target_os = "linux")]
pub (crate) mod mainloop_linux;

pub mod mainloop;
pub mod interface;
