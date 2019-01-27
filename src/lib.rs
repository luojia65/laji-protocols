pub mod discard;

#[cfg(not(feature = "mio-powered"))]
#[path = "daytime-threads.rs"]
pub mod daytime;
#[cfg(feature = "mio-powered")]
#[path = "daytime-mio.rs"]
pub mod daytime;
