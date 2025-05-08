use core::ffi::c_void;

use axerrno::{LinuxError, LinuxResult};
use axprocess::{Process, ProcessGroup, Pid};
use axsignal::{SignalSet, Signo, SignalInfo};
use axtask::{TaskExtRef, current};
use linux_raw_sys::general::{kernel_sigaction, SI_USER};
use starry_core::task::{ProcessData, get_process, processes, get_process_group};

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

pub fn send_signal_process(proc: &Process, sig: SignalInfo) {
    let Some(proc_data) = proc.data::<ProcessData>() else {
        return;
    };
    proc_data.signal_manager.send_signal(sig);
}

pub fn send_signal_process_group(pg: &ProcessGroup, sig: SignalInfo) -> usize {
    let processes = pg.processes();
    for proc in &processes {
        send_signal_process(proc, sig.clone());
    }
    processes.len()
}

fn make_siginfo(signo: i32, code: u32) -> LinuxResult<Option<SignalInfo>> {
    if signo == 0 {
        return Ok(None);
    }
    if !(1..64).contains(&signo) {
        return Err(LinuxError::EINVAL);
    }
    Ok(Some(SignalInfo::new(signo.into(), code.into())))
}

pub fn sys_kill(pid: i32, sig: i32) -> LinuxResult<isize> {
    let Some(sig) = make_siginfo(sig, SI_USER)? else {
        // TODO: should also check permissions
        return Ok(0);
    };

    let curr = current();
    match pid {
        1.. => {
            let proc = get_process(pid as Pid)?;
            send_signal_process(&proc, sig);
            Ok(1)
        }
        0 => {
            let pg = curr.task_ext().thread.process().group();
            let count = send_signal_process_group(&pg, sig);
            Ok(count as isize)
        }
        -1 => {
            let mut count = 0;
            for proc in processes() {
                if proc.is_init() {
                    // init process
                    continue;
                }
                send_signal_process(&proc, sig.clone());
                count += 1;
            }
            Ok(count)
        }
        ..-1 => {
            let pg = get_process_group((-pid) as Pid)?;
            let count = send_signal_process_group(&pg, sig);
            Ok(count as isize)
        }
    }
}