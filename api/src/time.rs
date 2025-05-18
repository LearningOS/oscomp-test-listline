use axhal::time::TimeValue;
use linux_raw_sys::general::{
    __kernel_old_timespec, __kernel_old_timeval, __kernel_sock_timeval, __kernel_timespec,
    timespec, timeval,
};

/// A helper trait for converting from and to `TimeValue`.
pub trait TimeValueLike {
    /// Converts from `TimeValue`.
    fn from_time_value(tv: TimeValue) -> Self;

    /// Converts to `TimeValue`.
    fn to_time_value(self) -> TimeValue;
}

impl TimeValueLike for TimeValue {
    fn from_time_value(tv: TimeValue) -> Self {
        tv
    }

    fn to_time_value(self) -> TimeValue {
        self
    }
}

impl TimeValueLike for timespec {
    fn from_time_value(tv: TimeValue) -> Self {
        Self {
            tv_sec: tv.as_secs() as _,
            tv_nsec: tv.subsec_nanos() as _,
        }
    }

    fn to_time_value(self) -> TimeValue {
        TimeValue::new(self.tv_sec as u64, self.tv_nsec as u32)
    }
}

impl TimeValueLike for __kernel_timespec {
    fn from_time_value(tv: TimeValue) -> Self {
        Self {
            tv_sec: tv.as_secs() as _,
            tv_nsec: tv.subsec_nanos() as _,
        }
    }

    fn to_time_value(self) -> TimeValue {
        TimeValue::new(self.tv_sec as u64, self.tv_nsec as u32)
    }
}

impl TimeValueLike for __kernel_old_timespec {
    fn from_time_value(tv: TimeValue) -> Self {
        Self {
            tv_sec: tv.as_secs() as _,
            tv_nsec: tv.subsec_nanos() as _,
        }
    }

    fn to_time_value(self) -> TimeValue {
        TimeValue::new(self.tv_sec as u64, self.tv_nsec as u32)
    }
}

impl TimeValueLike for timeval {
    fn from_time_value(tv: TimeValue) -> Self {
        Self {
            tv_sec: tv.as_secs() as _,
            tv_usec: tv.subsec_micros() as _,
        }
    }

    fn to_time_value(self) -> TimeValue {
        TimeValue::new(self.tv_sec as u64, self.tv_usec as u32 * 1000)
    }
}

impl TimeValueLike for __kernel_old_timeval {
    fn from_time_value(tv: TimeValue) -> Self {
        Self {
            tv_sec: tv.as_secs() as _,
            tv_usec: tv.subsec_micros() as _,
        }
    }

    fn to_time_value(self) -> TimeValue {
        TimeValue::new(self.tv_sec as u64, self.tv_usec as u32 * 1000)
    }
}

impl TimeValueLike for __kernel_sock_timeval {
    fn from_time_value(tv: TimeValue) -> Self {
        Self {
            tv_sec: tv.as_secs() as _,
            tv_usec: tv.subsec_micros() as _,
        }
    }

    fn to_time_value(self) -> TimeValue {
        TimeValue::new(self.tv_sec as u64, self.tv_usec as u32 * 1000)
    }
}

pub fn timespec_to_timevalue(ts: timespec) -> TimeValue {
    TimeValue::new(ts.tv_sec as u64, ts.tv_nsec as u32)
}

pub fn timevalue_to_timeval(tv: TimeValue) -> timeval {
    timeval {
        tv_sec: tv.as_secs() as _,
        tv_usec: tv.subsec_micros() as _,
    }
}

pub fn timeval_to_timevalue(tv: timeval) -> TimeValue {
    TimeValue::new(tv.tv_sec as u64, tv.tv_usec as u32 * 1_000)
}

pub fn timevalue_to_timespec(tv: TimeValue) -> timespec {
    timespec {
        tv_sec: tv.as_secs() as _,
        tv_nsec: tv.subsec_nanos() as _,
    }
}