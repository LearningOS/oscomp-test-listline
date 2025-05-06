# StarryX-Record

启动流程 _start(axhal) -> rust_entry(axhal) -> rust_main(axruntime) -> main(starry) -> run_user_app


run_user_app major process
1. 进入执行程序目录
2. 加载用户elf文件
3. 初始化用户上下文
4. 创建用户任务和线程数据
5. 复制全局命名空间数据到本地线程空间
6. 创建进程和线程
7. 阻塞主任务并调度

## 进程管理

```mermaid
graph TD
    subgraph "Task 层 (基础调度单元)"
        Task["Task/TaskInner
        - id: TaskId
        - name: String
        - state: TaskState
        - ctx: TaskContext
        - kstack: TaskStack
        - task_ext: AxTaskExt"]
    end
    
    subgraph "Task扩展层 (连接Task和Thread)"
        TaskExt["TaskExt
        - time: TimeStat
        - thread: Arc<Thread>"]
    end
    
    subgraph "Thread 层 (线程)"
        Thread["Thread
        - tid: Pid
        - process: Arc<Process>
        - data: Box<dyn Any>"]
        
        ThreadData["ThreadData
        - clear_child_tid: AtomicUsize"]
    end
    
    subgraph "Process 层 (进程)"
        Process["Process
        - pid: Pid
        - is_zombie: AtomicBool
        - tg: ThreadGroup
        - data: Box<dyn Any>
        - children: StrongMap<Pid, Arc<Process>>
        - parent: Weak<Process>
        - group: Arc<ProcessGroup>"]
        
        ProcessData["ProcessData
        - exe_path: String
        - aspace: Arc<Mutex<AddrSpace>>
        - ns: AxNamespace
        - heap_bottom/top: AtomicUsize"]
        
        ThreadGroup["ThreadGroup
        - threads: WeakMap<Pid, Weak<Thread>>
        - exit_code: i32
        - group_exited: bool"]
        
        ProcessGroup["ProcessGroup
        - pgid: Pid
        - session: Arc<Session>
        - processes: WeakMap<Pid, Weak<Process>>"]
        
        Session["Session
        - sid: Pid
        - process_groups: WeakMap<Pid, Weak<ProcessGroup>>"]
    end
    
    %% 连接关系
    Task -->|拥有| TaskExt
    TaskExt -->|引用| Thread
    Thread -->|属于| Process
    Thread -->|拥有| ThreadData
    Process -->|拥有| ProcessData
    Process -->|管理| ThreadGroup
    ThreadGroup -->|包含| Thread
    Process -->|归属于| ProcessGroup
    ProcessGroup -->|归属于| Session
    Process -->|子进程关系| Process
```



```rust
axprocess::Process::new_init(axtask::current().id().as_u64() as _).build();
```

相关数据结构

```rust
/// A builder for creating a new [`Process`].
pub struct ProcessBuilder {
    pid: Pid,
    parent: Option<Arc<Process>>,
    data: Box<dyn Any + Send + Sync>,
}

/// A process.
pub struct Process {
    pid: Pid,
    is_zombie: AtomicBool,
    pub(crate) tg: SpinNoIrq<ThreadGroup>,

    data: Box<dyn Any + Send + Sync>,

    // TODO: child subreaper
    children: SpinNoIrq<StrongMap<Pid, Arc<Process>>>,
    parent: SpinNoIrq<Weak<Process>>,

    group: SpinNoIrq<Arc<ProcessGroup>>,
}

pub(crate) struct ThreadGroup {
    pub(crate) threads: WeakMap<Pid, Weak<Thread>>,
    pub(crate) exit_code: i32,
    pub(crate) group_exited: bool,
}

/// A [`ProcessGroup`] is a collection of [`Process`]es.
pub struct ProcessGroup {
    pgid: Pid,
    pub(crate) session: Arc<Session>,
    pub(crate) processes: SpinNoIrq<WeakMap<Pid, Weak<Process>>>,
}

/// A [`Session`] is a collection of [`ProcessGroup`]s.
pub struct Session {
    sid: Pid,
    pub(crate) process_groups: SpinNoIrq<WeakMap<Pid, Weak<ProcessGroup>>>,
    // TODO: shell job control
}
```

## 信号机制

````mermaid
classDiagram
    class Signo {
        +SIGHUP, SIGINT, SIGKILL, etc.
        +is_realtime() bool
        +default_action() DefaultSignalAction
    }
    
    class DefaultSignalAction {
        <<enumeration>>
        Terminate
        Ignore
        CoreDump
        Stop
        Continue
    }
    
    class SignalOSAction {
        <<enumeration>>
        Terminate
        CoreDump
        Stop
        Continue
        Handler
    }
    
    class SignalSet {
        -u64 value
        +add(signal: Signo) bool
        +remove(signal: Signo) bool
        +has(signal: Signo) bool
        +dequeue(mask: SignalSet) Option~Signo~
        +to_ctype(dest: kernel_sigset_t)
    }
    
    class SignalInfo {
        -siginfo_t raw_info
        +new(signo: Signo, code: u32)
        +signo() Signo
        +set_signo(signo: Signo)
        +code() u32
        +set_code(code: u32)
    }
    
    class SignalActionFlags {
        <<bitflags>>
        +SIGINFO
        +NODEFER
        +RESETHAND
        +RESTART
        +ONSTACK
        +RESTORER
    }
    
    class SignalDisposition {
        <<enumeration>>
        Default
        Ignore
        Handler
    }
    
    class SignalAction {
        +flags: SignalActionFlags
        +mask: SignalSet
        +disposition: SignalDisposition
        +restorer: __sigrestore_t
        +to_ctype(dest: kernel_sigaction)
    }
    
    class SignalStack {
        +sp: usize
        +flags: u32
        +size: usize
        +disabled() bool
    }
    
    class PendingSignals {
        +set: SignalSet
        -info_std: [Option~SignalInfo~; 32]
        -info_rt: [VecDeque~SignalInfo~; 33]
        +new()
        +put_signal(sig: SignalInfo) bool
        +dequeue_signal(mask: SignalSet) Option~SignalInfo~
    }
    
    Signo --> DefaultSignalAction: defines default action
    SignalDisposition --> Signo: references for handlers
    SignalInfo --> Signo: contains signal number
    SignalSet --> Signo: manages set of signals
    SignalAction --> SignalDisposition: defines action
    SignalAction --> SignalSet: holds blocked signals
    SignalAction --> SignalActionFlags: configures behavior
    PendingSignals --> SignalSet: tracks pending signals
    PendingSignals --> SignalInfo: stores signal info
```

````



### 注册信号

#### Linux

```c
struct sigaction {
#ifndef __ARCH_HAS_IRIX_SIGACTION (mips define)
	__sighandler_t	sa_handler;
	unsigned long	sa_flags;
#else
	unsigned int	sa_flags;
	__sighandler_t	sa_handler;
#endif
#ifdef __ARCH_HAS_SA_RESTORER (x86)
	__sigrestore_t sa_restorer;
#endif
	sigset_t	sa_mask;	/* mask last for extensibility */
};

struct k_sigaction {
	struct sigaction sa;
#ifdef __ARCH_HAS_KA_RESTORER
	__sigrestore_t ka_restorer;
#endif
};

SYSCALL_DEFINE4(rt_sigaction, int, sig,
    const struct sigaction __user *, act,
    struct sigaction __user *, oact,
    size_t, sigsetsize)
{
    struct k_sigaction new_sa, old_sa;
    int ret = -EINVAL;
......
    if (act) {
      if (copy_from_user(&new_sa.sa, act, sizeof(new_sa.sa)))
        return -EFAULT;
    }

    ret = do_sigaction(sig, act ? &new_sa : NULL, oact ? &old_sa : NULL);

    if (!ret && oact) {
        if (copy_to_user(oact, &old_sa.sa, sizeof(old_sa.sa)))
            return -EFAULT;
    }
out:
    return ret;
}

int do_sigaction(int sig, struct k_sigaction *act, struct k_sigaction *oact)
{
    struct task_struct *p = current, *t;
    struct k_sigaction *k;
    sigset_t mask;
......
    k = &p->sighand->action[sig-1];

    spin_lock_irq(&p->sighand->siglock);
    if (oact)
        *oact = *k;

    if (act) {
        sigdelsetmask(&act->sa.sa_mask, sigmask(SIGKILL) | sigmask(SIGSTOP));
        *k = *act;
......
  }

  spin_unlock_irq(&p->sighand->siglock);
  return 0;
}
```

### 发送信号

## ArceOS change

### axhal

```rust
// arch/loongarch64
pub use self::context::{TaskContext, TrapFrame, GeneralRegisters};
```

