use axerrno::{LinuxError, LinuxResult};
use axprocess::{Pid, Process, ProcessGroup, Thread};
use axsignal::{SignalInfo, SignalSet, Signo};
use axtask::{TaskExtRef, current};
use linux_raw_sys::general::{
    SI_TKILL, SI_USER, SIG_BLOCK, SIG_SETMASK, SIG_UNBLOCK, kernel_sigaction, timespec,
};
use starry_core::task::{
    ProcessData, ThreadData, get_process, get_process_group, get_thread, processes,
};

use crate::ptr::{PtrWrapper, UserConstPtr, UserPtr};

pub fn check_sigsetsize(sigsetsize: usize) -> LinuxResult<()> {
    if sigsetsize != core::mem::size_of::<SignalSet>() {
        return Err(LinuxError::EINVAL);
    }
    Ok(())
}

pub fn sys_rt_sigprocmask(
    how: i32,
    set: UserConstPtr<SignalSet>,
    oldset: UserPtr<SignalSet>,
    sigsetsize: usize,
) -> LinuxResult<isize> {
    check_sigsetsize(sigsetsize)?;

    let curr = current();
    let mut blocked = curr.task_ext().thread_data().signal_manager.blocked_lock();

    if let Some(oldset) = oldset.nullable(|oldset| oldset.get())? {
        unsafe { *oldset = *blocked };
    }

    if let Some(set) = set.nullable(|set| set.get())? {
        match how as u32 {
            SIG_BLOCK => unsafe { blocked.add_from(&*set) },
            SIG_UNBLOCK => unsafe { blocked.remove_from(&*set) },
            SIG_SETMASK => unsafe { *blocked = *set },
            _ => return Err(LinuxError::EINVAL),
        }
    }
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

pub fn sys_rt_sigreturn() -> LinuxResult<isize> {
    warn!("sys_rt_sigpending: not implemented");
    Ok(0)
}

pub fn sys_rt_sigpending() -> LinuxResult<isize> {
    warn!("sys_rt_sigpending: not implemented");
    Ok(0)
}

pub fn sys_rt_sigtimedwait(
    set: UserConstPtr<SignalSet>,
    info: UserPtr<SignalInfo>,
    timeout: UserConstPtr<timespec>,
    sigsetsize: usize,
) -> LinuxResult<isize> {
    debug!("sys_rt_sigtimedwait");
    check_sigsetsize(sigsetsize)?;

    let set = set.get_as_null_terminated()?;
    let info_ptr = info.nullable(|info| info.get())?;

    let timeout_duration = if let Some(timeout) = timeout.nullable(|timeout| timeout.get())? {
        unsafe {
            let seconds = (*timeout).tv_sec as u64;
            let nanos = (*timeout).tv_nsec as u32;
            Some(core::time::Duration::new(seconds, nanos))
        }
    } else {
        None
    };

    let curr = current();
    let thr_data = curr.task_ext().thread_data();

    let siginfo = thr_data
        .signal_manager
        .wait_timeout(unsafe { *set.as_ptr() }, timeout_duration);

    match siginfo {
        Some(sig) => {
            if let Some(info_ptr) = info_ptr {
                unsafe { *info_ptr = sig.clone() };
            }
            Ok(sig.signo() as isize)
        }
        None => Err(LinuxError::EAGAIN),
    }
}

pub fn sys_rt_sigqueueinfo() -> LinuxResult<isize> {
    warn!("sys_rt_sigqueueinfo: not implemented");
    Ok(0)
}

pub fn sys_rt_sigsuspend() -> LinuxResult<isize> {
    warn!("sys_rt_sigsuspend: not implemented");
    Ok(0)
}

pub fn sys_sigaltstack() -> LinuxResult<isize> {
    warn!("sys_sigaltstack: not implemented");
    Ok(0)
}

pub fn send_signal_thread(thread: &Thread, sig: SignalInfo) {
    let Some(thread_data) = thread.data::<ThreadData>() else {
        return;
    };
    thread_data.signal_manager.send_signal(sig);
}

pub fn send_signal_process(proc: &Process, sig: SignalInfo) {
    info!(
        "Send signal {} to process {}",
        sig.signo() as i32,
        proc.pid()
    );
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
    Ok(Some(SignalInfo::new(signo.into(), code)))
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

pub fn sys_tkill(tid: i32, sig: i32) -> LinuxResult<isize> {
    let Some(sig) = make_siginfo(sig, SI_USER)? else {
        // TODO: should also check permissions
        return Ok(0);
    };
    let thread = get_thread(tid as Pid)?;
    send_signal_thread(&thread, sig);
    Ok(0)
}

pub fn sys_tgkill(tgid: i32, tid: i32, sig: i32) -> LinuxResult<isize> {
    let Some(sig) = make_siginfo(sig, SI_TKILL as u32)? else {
        // TODO: should also check permissions
        return Ok(0);
    };

    let thr = get_thread(tid as Pid)?;
    if thr.process().pid() != tgid as Pid {
        return Err(LinuxError::ESRCH);
    }
    send_signal_thread(&thr, sig);
    Ok(0)
}
