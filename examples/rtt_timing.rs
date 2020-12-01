//! examples/rtt_timing.rs

//#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

use panic_halt as _;
use cortex_m::{asm, peripheral::DWT};

#[rtic::app(device = stm32f4)]
const APP: () = {
    #[init]
    fn init(mut cx: init::Context) {
        // Initialize (enable) the monotonic timer (CYCCNT)
        cx.core.DWT.enable_cycle_counter();

        unsafe {
            cx.core.DWT.cyccnt.write(0);
        }

        let (start, end) = timed_loop();
        let _diff = end.wrapping_sub(start);
    }

    #[idle]
    fn idle(_cx: idle::Context) -> ! {
        loop {
            continue;
        }
    }
};

// Forbid inlining and keeping name (symbol) readable.
#[inline(never)]
#[no_mangle]
fn timed_loop() -> (u32, u32) {
    let start = DWT::get_cycle_count();
    for _ in 0..10000 {
        asm::nop();
    }
    let end = DWT::get_cycle_count();
    (start, end)
}

// We can measure execution time using the built in cycle counter,
// a wrapping 32 bit timer.
//
// `DWT::get_cycle_count()` reads the current value.
//
// ------------------------------------------------------------------------
// Exercises:
//
// A.1) What is the cycle count for the loop?
// > cargo run --example rtt_timing
//
// ANSWER: diff 5300151 
//
// A.2) How many cycles per iteration?
//
// ANSWER: 530
//
// A.3) Why do we need a wrapping subtraction?
//
// ANSWER: So it does not underflow and instead wraps back to its upper bound 
//
// ------------------------------------------------------------------------
// Now try a release (optimized build, see `Cargo.toml` for build options).
// B.1) What is the cycle count for the loop?
// > cargo run --example rtt_timing --release
//
// [Your answer here]
//
// B.2) How many cycles per iteration?
//
// ANSWER: 7 
//
// What is the speedup (A/B)?
//
// ANSWER: 76x faster 
//
// Why do you think it differs that much?
//
// ANSWER: The compiler does a lot more optimization in a release. Exactly
// what and where it does it, I couldn't guess.
//
// ------------------------------------------------------------------------
// In the loop there is just a single assembly instruction (nop).
// However, the API for "inline assembly" is not yet stabilized, thus
// we have to call an "external" functions that implements the assembly.
//
// We can switch to the "nightly" channel (which allows experimental features).
//
// > rustup override set nightly
// info: using existing install for 'nightly-x86_64-unknown-linux-gnu'
// info: override toolchain for '/home/pln/courses/e7020e/app' set to 'nightly-x86_64-unknown-linux-gnu'
//
// nightly-x86_64-unknown-linux-gnu unchanged - rustc 1.49.0-nightly (cf9cf7c92 2020-11-10)
//
// > rustup target add thumbv7em-none-eabi
// (only needed first time)
//
// Now try a release (optimized build, see `Cargo.toml` for build options).
// C.1) What is the cycle count for the loop?
// > cargo run --example rtt_timing --release --features nightly
//
// [Your answer here]
//
// C.2) How many cycles per iteration?
//
// ANSWER: 4 
//
// What is the speedup (A/C)?
//
// ANSWER: 133x faster
//
// ------------------------------------------------------------------------
// D) Now lets have a closer look at the generated assembly.
//
// > crgo objdump --example rtt_timing --release  --features nightly -- --disassemble --no-show-raw-insn > rtt_timing.objdump
//
// Open the file in you editor and search for the `timed_loop`.
//
// [Assembly for function `timed_loop` here]
// 08000232 timed_loop:
//   1  8000232:       movw    r1, #4100
//   2  8000236:       movw    r2, #10000
//   3  800023a:       movt    r1, #57344
//   4  800023e:       ldr     r0, [r1]
//   5  8000240:       subs    r2, #1
//   6  8000242:       nop
//   7  8000244:       bne     #-8 <timed_loop+0xe>
//   8  8000246:       ldr     r1, [r1]
//   9  8000248:       bx      lr
//
// Locate the loop body, and verify that it makes sense
// based on the information from the technical documentation:
//
// https://developer.arm.com/documentation/ddi0439/b/Programmers-Model/Instruction-set-summary/Cortex-M4-instructions
//
//
// ------------------------------------------------------------------------
// E) Now we shall take detailed control over the debugging.
//
// Alter the .cargo/config
// uncomment:
// runner = "arm-none-eabi-gdb -q -x openocd.gdb"
// or (if under ubuntu)
// runner = "gdb-multiarch -q -x openocd.gdb"
//
// comment:
// # runner = "probe-run --chip STM32F411RETx"
//
// Remember to save the file!
//
// In a separate terminal start `openocd`.
// > openocd -f openocd.cfg
// And in the main terminal run:
// > cargo run --example rtt_timing --release --features nightly
// ...
// halted: PC: 0x0800019a
// 0x0800019a in cortex_m_rt::Reset () at /home/pln/.cargo/registry/src/github.com-1ecc6299db9ec823/cortex-m-rt-0.6.13/src/lib.rs:497
// 497     pub unsafe extern "C" fn Reset() -> ! {
//
// Now add a breakpoint at `timed_loop`.
// (gdb) break timed_loop
// Breakpoint 5 at 0x800023e: file examples/rtt_timing.rs, line 46.
//
// Now we can continue (first break will be at `init`)
// (gdb) continue
// Breakpoint 4, cortex_m::interrupt::disable () at /home/pln/.cargo/registry/src/github.com-1ecc6299db9ec823/cortex-m-0.6.4/src/interrupt.rs:13
//                  llvm_asm!("cpsid i" ::: "memory" : "volatile");
// (`init` in RTIC runs with interrupts disabled)
// (gdb) continue
// Breakpoint 5, core::ptr::read_volatile<u32> (src=<optimized out>) at /home/pln/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/ptr/mod.rs:1052
//        unsafe { intrinsics::volatile_load(src) }
//
// Now we are inside the `timed_loop`.
// (gdb) disassemble
// Dump of assembler code for function _ZN10rtt_timing10timed_loop17h4a445e5f592f3304E:
// 0x08000232 <+0>:     movw    r1, #4100       ; 0x1004
// 0x08000236 <+4>:     movw    r2, #10000      ; 0x2710
// 0x0800023a <+8>:     movt    r1, #57344      ; 0xe000
// => 0x0800023e <+12>:    ldr     r0, [r1, #0]
// 0x08000240 <+14>:    subs    r2, #1
// 0x08000242 <+16>:    nop
// 0x08000244 <+18>:    bne.n   0x8000240 <rtt_timing::timed_loop+14>
// 0x08000246 <+20>:    ldr     r1, [r1, #0]
// 0x08000248 <+22>:    bx      lr
//
// (breakpoints can be a bit "off" for release builds, here it
// put the breakpoint not at the function entry but rather at the
// first Rust line (the load done by `DWT::get_cycle_count()`)).
//
// Is there a function call to the DWT::get_cycle_count(),
// if not can you explain what has happened here?
// In particular, what is the content of r1?
//
// Confer to the documentation:
// https://developer.arm.com/documentation/ddi0439/b/Data-Watchpoint-and-Trace-Unit/DWT-Programmers-Model
//
// ANSWER: There is no function call here. It seems it might have been replaced with the contets
// of register r1 which contains the int 4100. Which might be the initial cycle count.
//
// Now check your answer by dumping the registers
// (gdb) info registers
//
// [Register dump here]
//  r0             0x80000000          -2147483648
//  r1             0xe0001004          -536866812
//  r2             0x2710              10000
//  r3             0xa                 10
//  r4             0x20000000          536870912
//  r5             0x20000430          536871984
//  r6             0x0                 0
//  r7             0x2000ffe8          536936424
//  r8             0x0                 0
//  r9             0x0                 0
//  r10            0x0                 0
//  r11            0x0                 0
//  r12            0x1                 1
//  sp             0x2000ff98          0x2000ff98
//  lr             0x8000373           134218611
//  pc             0x800023e           0x800023e <rtt_timing::timed_loop+12>
//
// We can now set a breakpoint exactly at the `nop`.
//
// (gdb) break *0x8000242
//
// And continue execution to the breakpoint:
// (gdb) continue
//
// (gdb) disassemble
//    0x08000232 <+0>:     movw    r1, #4100       ; 0x1004
// 0x08000236 <+4>:     movw    r2, #10000      ; 0x2710
// 0x0800023a <+8>:     movt    r1, #57344      ; 0xe000
// 0x0800023e <+12>:    ldr     r0, [r1, #0]
// 0x08000240 <+14>:    subs    r2, #1
// => 0x08000242 <+16>:    nop
// 0x08000244 <+18>:    bne.n   0x8000240 <rtt_timing::timed_loop+14>
// 0x08000246 <+20>:    ldr     r1, [r1, #0]
// 0x08000248 <+22>:    bx      lr
//
// You can inspect the memory directly.
// What is the current value of the cycle counter?
//
// (gdb) x 0xe0001004
//
// ANSWER: 0x60bb2f91 -> 1622880145
//
// Now, let's execute one iteration:
// (gdb) continue
//
// What is now the current value of the cycle counter?
//
// ANSWER: 0x60bb2f95 -> 1622880149
//
// By how much does the cycle counter increase for each iteration?
//
// ANSWER: By 4
//
// ------------------------------------------------------------------------
// F) Reseting the cycle counter
// You can restart the program by a "soft reset".
// (gdb) monitor reset init
// "monitor" the command to be sent to `openocd`
// "reset init" will reset the MCU and run a the "init" hook
// in the currently loaded `openocd` configuration.
//
// Now repeat the procedure from E) and check the value of
// the cycle counter.
//
// You will find that its not starting over but continuing.
// This may be a bit confusing (we like deterministic behavior).
//
// Look in the documentation for `cortex_m`
// https://docs.rs/cortex-m/0.6.4/cortex_m/peripheral/dwt/struct.RegisterBlock.html
//
// This is a "raw" API to the underlying hardware.
// Under the hood, it uses `volatile_register`
// https://docs.rs/volatile-register/0.2.0/volatile_register/struct.RW.html
//
// for which write is marked "unsafe".
//
// Figure out the way to access the register through:
// `cx.core.DWT.cyccnt.write(0)`.
// And add that to the code just before calling `timed_loop`.
//
// (Remember to comment out #![deny(unsafe_code)])
//
// Now run re-run the procedure from E)
//
// What is the initial value of the cycle counter
// (when hitting the `timed_loop` breakpoint)?
//
// ANSWER: 0x00000185 -> 389
//
// ------------------------------------------------------------------------
// F) Finally some statics
// For embedded it is often/typically crucial to reduce overhead,
// - code footprint (flash memory)
// - memory footprint (sram memory)
// - execution OH (speed/power consumption)
//
// We have already looked execution overhead and can conclude
// Rust to do an excellent job.
//
// The ram requirement for the application amounts only to
// to the buffers (allocated for RTT), and the stack
// for the functions (which is in this case 0 for the `timed_loop`).
// (Look at the code, there is no pushing/popping, no local stack.)
//
// Let's have a look at the code footprint (flash).
//
// > cargo bloat -n 100 --example rtt_timing --release --features nightly
// ...
// 0.0%   0.4%    24B   [Unknown] timed_loop
// ...
//
// That is, the timed loop takes only 24 bytes of memory!
//
// > cargo size --example rtt_timing --release --features nightly
// Finished release [optimized + debuginfo] target(s) in 0.03s
// text    data     bss     dec     hex filename
// 6876       0    1084    7960    1f18 rtt_timing
//
// And the code overall less than 8k of flash.
//
// Your assignment now is to get this down, by identifying the
// memory hogs. You can remove the tracing, replace the panic
// handler by `panic_halt`, but besides that it should still
// measure time (debugging in gdb of `timed_loop` must still work).
//
// > cargo size --example rtt_timing --release --features nightly
//
// ANSWER: Remove all tracing and using panic_halt: 
// text    data     bss     dec     hex filename
//  652       0       0     652     28c rtt_timing
//
//
// I was able to get down to:
// > cargo size --example rtt_timing --release --features nightly
// Compiling app v0.1.0 (/home/pln/courses/e7020e/app)
// Finished release [optimized + debuginfo] target(s) in 0.32s
// text    data     bss     dec     hex filename
// 660       0       0     660     294 rtt_timing
// Try beat that...
// ------------------------------------------------------------------------
// Summary:
// What are the learning outcomes.
//
// - You have confirmed that RTIC is extremely light weight
//   (zero-cost in release build).
//   Applications can be be less than 1k flash. Ideal for IoT!
//
// - You have seen that RTIC applications are easy to trace using RTT
//   If more details are required, you have learned to use gdb.
//
// - You have confirmed that Rust generates:
//   really, really, bad code in debug build (beware!).
//   really, really, really good code in release build!
//
// - You have setup timing measurements which are
//   Cycle accurate (0.000 000 01s at 100MHz).
//   Consistent (down to a single clock cycle).
//   Predictable (you got what you expected right?)
