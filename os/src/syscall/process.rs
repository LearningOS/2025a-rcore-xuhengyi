//! Process management syscalls
use crate::task::*;
use crate::mm::translated_byte_buffer;
use crate::timer::get_time_us;

use crate::mm::{PageTable, VirtAddr, PTEFlags};
use crate::config::MAX_SYSCALL_NUM;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    if _ts.is_null() {
        return -1;
    }

    let token = current_user_token();
    let len = core::mem::size_of::<TimeVal>();
    let mut buffers = translated_byte_buffer(token, _ts as *const u8, len);

    // 取得当前时间（微秒）
    let us = get_time_us();
    let tv = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };

    // 把 tv 按字节拷贝到用户缓冲区（可能跨页）
    let src_ptr = &tv as *const TimeVal as *const u8;
    let mut offset = 0;
    for buf in buffers.iter_mut() {
        if offset >= len {
            break;
        }
        let copy_len = core::cmp::min(len - offset, buf.len());
        let src_slice = unsafe {
            core::slice::from_raw_parts(src_ptr.add(offset), copy_len)
        };
        buf[..copy_len].copy_from_slice(src_slice);
        offset += copy_len;
    }

    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
trace!("kernel: sys_trace");
    match _trace_request {
        // 读取用户地址处一个字节
        0 => {
            let token = current_user_token();
            let page_table = PageTable::from_token(token);
            let va = VirtAddr::from(_id);
            let vpn = va.floor();
            let off = va.page_offset();

            let pte = match page_table.translate(vpn) {
                Some(p) => p,
                None => return -1,
            };
            let flags = pte.flags();
            // 必须是用户态可见且可读
            if !flags.contains(PTEFlags::U) || !flags.contains(PTEFlags::R) {
                return -1;
            }
            let ppn = pte.ppn();
            let bytes = ppn.get_bytes_array();
            bytes[off] as isize
        }

        // 向用户地址写入一个字节
        1 => {
            let token = current_user_token();
            let page_table = PageTable::from_token(token);
            let va = VirtAddr::from(_id);
            let vpn = va.floor();
            let off = va.page_offset();

            let pte = match page_table.translate(vpn) {
                Some(p) => p,
                None => return -1,
            };
            let flags = pte.flags();
            // 必须是用户态可见且可写
            if !flags.contains(PTEFlags::U) || !flags.contains(PTEFlags::W) {
                return -1;
            }
            let ppn = pte.ppn();
            let bytes = ppn.get_bytes_array();
            bytes[off] = (_data & 0xff) as u8;
            0
        }

        // 查询当前任务 syscall(id) 的调用次数（包括这次 sys_trace）
        2 => {
            if _id >= MAX_SYSCALL_NUM {
                return -1;
            }
            let cnt = current_syscall_times(_id);
            cnt as isize
        }

        _ => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    current_mmap(_start, _len, _port)
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    current_munmap(_start, _len)
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
