//! examples/timing_task.rs

#![deny(warnings)]
#![no_main]
#![no_std]

use cortex_m::{asm, peripheral::DWT};
use panic_halt as _;
use stm32f4::stm32f411;

#[rtic::app(device = stm32f411)]
const APP: () = {
    struct Resources {
        dwt: DWT,
    }

    #[init]
    fn init(mut cx: init::Context) -> init::LateResources {
        // Initialize (enable) the monotonic timer (CYCCNT)
        cx.core.DCB.enable_trace();
        cx.core.DWT.enable_cycle_counter();
        init::LateResources { dwt: cx.core.DWT }
    }

    #[idle(resources = [dwt])]
    fn idle(cx: idle::Context) -> ! {
        unsafe { cx.resources.dwt.cyccnt.write(0) };
        asm::bkpt();
        rtic::pend(stm32f411::Interrupt::EXTI0);
        asm::bkpt();
        loop {
            continue;
        }
    }

    #[task(binds = EXTI0)]
    fn exti0(_cx: exti0::Context) {
        asm::bkpt();
    }
};

// Now we are going to have a look at the scheduling of RTIC tasks
//
// First create an objdump file:
// >  cargo objdump --example timing_task --release  --features nightly -- --disassemble > timing_task.objdump
//
// Lookup the EXTI0 symbol (RTIC binds the exti0 task to the interrupt vector).
//
// You should find something like:
//
// 08000232 <EXTI0>:
//  8000232: 00 be        	bkpt	#0
//  8000234: 00 20        	movs	r0, #0
//  8000236: 80 f3 11 88  	msr	basepri, r0
//  800023a: 70 47        	bx	lr
//
// The application triggers the `exti0` task from `idle`, let's see
// how that pans out.
//
// > cargo run --example timing_task --release --features nightly
// Then continue to the first breakpoint instruction:
// (gdb) c
// timing_task::idle (cx=...) at examples/timing_task.rs:28
// 28              asm::bkpt();
//
// (gdb) x 0xe0001004
// 0
//
// Here we see, that we have successfully set the cycle counter to zero.
// The `rtic::pend(stm32f411::Interrupt::EXTI0)` "emulates" the
// arrival/triggering of an external interrupt associated with
// the `exti0` task.
//
// (gdb) c
// timing_task::APP::EXTI0 () at examples/timing_task.rs:11
// 11      #[rtic::app(device = stm32f411)]
//
// Since `exti0` has a default priority = 1, it will preempt `idle` (at priority = 0),
// and the debugger breaks in the `exti0` task.
// (Notice, RTIC translates logical priorities to hw priorities for you.)
//
// (gdb) x 0xe0001004
//
// ANSWER: 0x0000000b -> 0d11 
//
// (gdb) disassemble
//
// ANSWER:
// => 0x08000232 <+0>:     bkpt    0x0000
//    0x08000234 <+2>:     movs    r0, #0
//    0x08000236 <+4>:     msr     BASEPRI, r0
//    0x0800023a <+8>:     bx      lr
//
// You should see that we hit the breakpoint in `exti0`, and
// that the code complies to the objdump EXTI disassembly.
//
// Confer to the document:
// https://community.arm.com/developer/ip-products/processors/b/processors-ip-blog/posts/beginner-guide-on-interrupt-latency-and-interrupt-latency-of-the-arm-cortex-m-processors
//
// What was the software latency observed to enter the task?
//
// ANSWER: 11 clock cycles (0x0 -> 0xb).
//
// Does RTIC infer any overhead for launching the task?
//
// ANSWER: No because according to the URL above the M4 interrupt latency is typically 12 cycles. 
//
// Now we can continue to measure the round trip time.
//
// (gdb) c
// timing_resources::idle (cx=...) at examples/timing_resources.rs:34
// 34              asm::bkpt();
//
// (gdb) x 0xe0001004
//
// ANSWER: 0x00000017 -> 23 
//
// Looking at the EXTI0 (exti0) code, we see two additional
// instructions used to restore the BASEPRI register.
// This OH will be removed in next release of RTIC.
// So we can conclude RTIC to have a 2-cycle OH (in this case).
// (In the general case, as we will see later restoring BASEPRI
// is actually necessary so its just this corner case that is
// sub-optimal.)
