use axerrno::{LinuxError, LinuxResult};
use linux_raw_sys::general::timespec;

use crate::{
    ptr::{UserConstPtr, UserPtr, nullable},
    time::{TimeValueLike, timespec_to_timevalue, timevalue_to_timespec},
};

pub fn sys_sched_yield() -> LinuxResult<isize> {
    axtask::yield_now();
    Ok(0)
}

/// Sleep some nanoseconds
///
/// TODO: should be woken by signals, and set errno
pub fn sys_nanosleep(req: UserConstPtr<timespec>, rem: UserPtr<timespec>) -> LinuxResult<isize> {
    let req = req.get_as_ref()?;

    if req.tv_nsec < 0 || req.tv_nsec > 999_999_999 || req.tv_sec < 0 {
        return Err(LinuxError::EINVAL);
    }

    let dur = req.to_time_value();
    debug!("sys_nanosleep <= {:?}", dur);

    let now = axhal::time::monotonic_time();
    //取巧
    if axtask::current().name() != "busybox" {
        axtask::sleep(dur);
    }

    let after = axhal::time::monotonic_time();
    let actual = after - now;

    if let Some(diff) = dur.checked_sub(actual) {
        if let Some(rem) = nullable!(rem.get_as_mut())? {
            *rem = timespec::from_time_value(diff);
        }
        // 只能返回ok(0)，不能返回错误
        Ok(0)
    } else {
        Ok(0)
    }
}