
#[cfg(target_os = "windows")]
include!("channel_windows.rs");

#[cfg(target_os = "linux")]
include!("channel_linux.rs");

//include!("channel_refcell.rs");
include!("channel_rwlock.rs");


unsafe impl<T : std::marker::Send + 'static > Sync for EvtChannel<T> {}
unsafe impl<T : std::marker::Send + 'static > Send for EvtChannel<T> {}