use core::ffi::c_int;

use axerrno::LinuxResult;
use linux_raw_sys::general::{__kernel_fd_set, sigset_t, timespec, timeval};

use crate::UserPtr;

pub fn sys_select(
    _nfds: c_int,
    _readfds: UserPtr<__kernel_fd_set>,
    _writefds: UserPtr<__kernel_fd_set>,
    _exceptfds: UserPtr<__kernel_fd_set>,
    _timeout: UserPtr<timeval>,
) -> LinuxResult<isize> {
    todo!()
}

pub fn sys_pselect6(
    _nfds: c_int,
    _readfds: UserPtr<__kernel_fd_set>,
    _writefds: UserPtr<__kernel_fd_set>,
    _exceptfds: UserPtr<__kernel_fd_set>,
    _timeout: UserPtr<timespec>,
    _sigmask: UserPtr<sigset_t>,
) -> LinuxResult<isize> {
    todo!()
}
