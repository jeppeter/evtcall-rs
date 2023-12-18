


#[cfg(target_os = "linux")]
include!("sockhdl_linux.rs");

#[cfg(target_os = "windows")]
include!("sockhdl_windows.rs");