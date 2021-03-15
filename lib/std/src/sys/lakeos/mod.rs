#![allow(missing_docs, nonstandard_style)]

use crate::io::ErrorKind;

#[cfg(any(doc, target_os = "linux"))]
pub use crate::os::linux as platform;

#[cfg(all(not(doc), target_os = "android"))]
pub use crate::os::android as platform;
#[cfg(all(not(doc), target_os = "dragonfly"))]
pub use crate::os::dragonfly as platform;
#[cfg(all(not(doc), target_os = "emscripten"))]
pub use crate::os::emscripten as platform;
#[cfg(all(not(doc), target_os = "freebsd"))]
pub use crate::os::freebsd as platform;
#[cfg(all(not(doc), target_os = "fuchsia"))]
pub use crate::os::fuchsia as platform;
#[cfg(all(not(doc), target_os = "haiku"))]
pub use crate::os::haiku as platform;
#[cfg(all(not(doc), target_os = "illumos"))]
pub use crate::os::illumos as platform;
#[cfg(all(not(doc), target_os = "ios"))]
pub use crate::os::ios as platform;
#[cfg(all(not(doc), target_os = "l4re"))]
pub use crate::os::linux as platform;
#[cfg(all(not(doc), target_os = "macos"))]
pub use crate::os::macos as platform;
#[cfg(all(not(doc), target_os = "netbsd"))]
pub use crate::os::netbsd as platform;
#[cfg(all(not(doc), target_os = "openbsd"))]
pub use crate::os::openbsd as platform;
#[cfg(all(not(doc), target_os = "redox"))]
pub use crate::os::redox as platform;
#[cfg(all(not(doc), target_os = "solaris"))]
pub use crate::os::solaris as platform;
#[cfg(all(not(doc), target_os = "lakeos"))]
pub use crate::os::lakeos as platform;

// pub use self::rand::hashmap_random_keys;
// pub use libc::strlen;
pub fn strlen(ptr: *const i8) -> i32 {
    let mut cur_ptr = ptr;
    let mut len = 0;
    unsafe {
        while *cur_ptr != b'\0' as i8 {
            len += 1;
            cur_ptr = cur_ptr.offset(1);
        }
    }
    len
}

// #[macro_use]
// pub mod weak;

// pub mod android;
// pub mod args;
// pub mod cmath;
// pub mod condvar;
// pub mod env;
// pub mod ext;
// pub mod fd;
// pub mod fs;
pub mod io;
// #[cfg(target_os = "l4re")]
// mod l4re;
pub mod memchr;
pub mod mutex;
// #[cfg(not(target_os = "l4re"))]
// pub mod net;
// #[cfg(target_os = "l4re")]
// pub use self::l4re::net;
// pub mod os;
pub mod path;
// pub mod pipe;
// pub mod process;
// pub mod rand;
// pub mod rwlock;
// pub mod stack_overflow;
pub mod stdio;
// pub mod thread;
// pub mod thread_local_dtor;
// pub mod thread_local_key;
// pub mod time;
// pub mod urpc;

pub use crate::sys_common::os_str_bytes as os_str;

#[cfg(not(test))]
pub fn init() {
    naive::rt::initialize_mm();

    naive::rt::populate_app_cspace();

    naive::rt::initialize_vmspace();
}

// #[cfg(target_os = "android")]
// pub use crate::sys::android::signal;
// #[cfg(not(target_os = "android"))]
// pub use libc::signal;

// pub fn decode_error_kind(errno: i32) -> ErrorKind {
//     match errno as libc::c_int {
//         libc::ECONNREFUSED => ErrorKind::ConnectionRefused,
//         libc::ECONNRESET => ErrorKind::ConnectionReset,
//         libc::EPERM | libc::EACCES => ErrorKind::PermissionDenied,
//         libc::EPIPE => ErrorKind::BrokenPipe,
//         libc::ENOTCONN => ErrorKind::NotConnected,
//         libc::ECONNABORTED => ErrorKind::ConnectionAborted,
//         libc::EADDRNOTAVAIL => ErrorKind::AddrNotAvailable,
//         libc::EADDRINUSE => ErrorKind::AddrInUse,
//         libc::ENOENT => ErrorKind::NotFound,
//         libc::EINTR => ErrorKind::Interrupted,
//         libc::EINVAL => ErrorKind::InvalidInput,
//         libc::ETIMEDOUT => ErrorKind::TimedOut,
//         libc::EEXIST => ErrorKind::AlreadyExists,

//         // These two constants can have the same value on some systems,
//         // but different values on others, so we can't use a match
//         // clause
//         x if x == libc::EAGAIN || x == libc::EWOULDBLOCK => ErrorKind::WouldBlock,

//         _ => ErrorKind::Other,
//     }
// }

// #[doc(hidden)]
// pub trait IsMinusOne {
//     fn is_minus_one(&self) -> bool;
// }

// macro_rules! impl_is_minus_one {
//     ($($t:ident)*) => ($(impl IsMinusOne for $t {
//         fn is_minus_one(&self) -> bool {
//             *self == -1
//         }
//     })*)
// }

// impl_is_minus_one! { i8 i16 i32 i64 isize }

// pub fn cvt<T: IsMinusOne>(t: T) -> crate::io::Result<T> {
//     if t.is_minus_one() { Err(crate::io::Error::last_os_error()) } else { Ok(t) }
// }

// pub fn cvt_r<T, F>(mut f: F) -> crate::io::Result<T>
// where
//     T: IsMinusOne,
//     F: FnMut() -> T,
// {
//     loop {
//         match cvt(f()) {
//             Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
//             other => return other,
//         }
//     }
// }

// // On Unix-like platforms, libc::abort will unregister signal handlers
// // including the SIGABRT handler, preventing the abort from being blocked, and
// // fclose streams, with the side effect of flushing them so libc buffered
// // output will be printed.  Additionally the shell will generally print a more
// // understandable error message like "Abort trap" rather than "Illegal
// // instruction" that intrinsics::abort would cause, as intrinsics::abort is
// // implemented as an illegal instruction.
// pub fn abort_internal() -> ! {
//     unsafe { libc::abort() }
// }

// #[panic_handler]
// fn panic(info: &core::panic::PanicInfo) -> ! {

//     // use crate::prelude::*;
//     rustyl4api::kprintln!("Panic! {:?}", info);
//     loop {
//         // arch::wfe();
//     }
// }