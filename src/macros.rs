

#[cfg(target_os = "windows")]
include!("macros_windows.rs");

#[cfg(target_os = "linux")]
include!("macros_linux.rs");