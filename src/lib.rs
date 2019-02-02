pub mod discard;

#[cfg(not(feature = "mio-powered"))]
#[path = "daytime-threads.rs"]
mod daytime_impl;
#[cfg(feature = "mio-powered")]
#[path = "daytime-mio.rs"]
mod daytime_impl;

pub mod daytime {
    pub use super::daytime_impl::*;
}

pub mod simtcp;

pub mod rakping;
