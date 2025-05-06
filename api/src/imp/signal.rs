use core::ffi::c_void;

use axerrno::{LinuxError, LinuxResult};
use axprocess::Pid;
use axsignal::{SignalSet, Signo};
use axtask::{TaskExtRef, current};
use linux_raw_sys::general::kernel_sigaction;

use crate::ptr::{PtrWrapper, UserConstPtr, UserPtr};

pub fn check_sigsetsize(sigsetsize: usize) -> LinuxResult<()> {
    if sigsetsize != core::mem::size_of::<SignalSet>() {
        return Err(LinuxError::EINVAL);
    }
    Ok(())
}

pub fn sys_rt_sigprocmask(
    _how: i32,
    _set: UserConstPtr<c_void>,
    _oldset: UserPtr<c_void>,
    _sigsetsize: usize,
) -> LinuxResult<isize> {
    warn!("sys_rt_sigprocmask: not implemented");
    Ok(0)
}

pub fn sys_rt_sigaction(
    signum: i32,
    act: UserConstPtr<kernel_sigaction>,
    oldact: UserPtr<kernel_sigaction>,
    sigsetsize: usize,
) -> LinuxResult<isize> {
    check_sigsetsize(sigsetsize)?;

    if !(1..=64).contains(&signum) {
        return Err(LinuxError::EINVAL);
    }
    if signum == Signo::SIGKILL || signum == Signo::SIGSTOP {
        return Err(LinuxError::EINVAL);
    }

    let curr = current();
    let mut actions = curr.task_ext().process_data().signal_manager.actions.lock();

    if let Some(oldact) = oldact.nullable(|oldact| oldact.get())? {
        actions[signum.into()].to_ctype(unsafe { &mut *oldact });
    }

    if let Some(act) = act.nullable(|act| act.get())? {
        actions[signum.into()] = unsafe { *act }.try_into()?;
    }

    Ok(0)
}

pub fn sys_sigtimedwait(
    _set: UserConstPtr<c_void>,
    _timeout: UserConstPtr<c_void>,
    _sigsetsize: usize,
) -> LinuxResult<isize> {
    warn!("sys_sigtimedwait: not implemented");
    Ok(0)
}

pub fn sys_kill(_pid: Pid, _sig: i32) -> LinuxResult<isize> {
    warn!("sys_kill: not implemented");
    Ok(0)
}
