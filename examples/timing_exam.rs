//! examples/timing_exam.rs

// #![deny(unsafe_code)]
// #![deny(warnings)]
#![no_main]
#![no_std]

use cortex_m::{asm, peripheral::DWT};
use panic_halt as _;
use rtic::cyccnt::{Duration, Instant, U32Ext};
use stm32f4::stm32f411;

#[no_mangle]
static mut T1_MAX_RP: u32 = 0;
#[no_mangle]
static mut T2_MAX_RP: u32 = 0;
#[no_mangle]
static mut T3_MAX_RP: u32 = 0;

#[rtic::app(device = stm32f411, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {
    struct Resources {
        #[init(0)]
        R1: u64, // non atomic data
        #[init(0)]
        R2: u64, // non atomic data
    }

    #[init(schedule = [t1, t2, t3])]
    fn init(mut cx: init::Context) {
        // Initialize (enable) the monotonic timer (CYCCNT)
        cx.core.DCB.enable_trace();
        cx.core.DWT.enable_cycle_counter();
        cx.schedule.t1(cx.start + 100_000.cycles()).unwrap();
        cx.schedule.t2(cx.start + 200_000.cycles()).unwrap();
        cx.schedule.t3(cx.start + 50_000.cycles()).unwrap();
    }

    // Deadline 100, Inter-arrival 100
    #[inline(never)]
    #[task(schedule = [t1], priority = 1)]
    fn t1(cx: t1::Context) {
        asm::bkpt();
        cx.schedule.t1(cx.scheduled + 100_000.cycles()).unwrap();
        asm::bkpt();
        
        let start = DWT::get_cycle_count();
        // emulates timing behavior of t1
        cortex_m::asm::delay(10_000);
        let end = DWT::get_cycle_count();
        let diff = end.wrapping_sub(start);

        // 2) your code here to update T1_MAX_RP and
        // break if deadline missed
        unsafe {
            if diff > T1_MAX_RP {
                T1_MAX_RP = diff;
            }
            if T1_MAX_RP > 100000 {
                panic!("T1 deadline missed");
            }
        }
    }

    // Deadline 200, Inter-arrival 200
    #[inline(never)]
    #[task(schedule = [t2], resources = [R1, R2], priority = 2)]
    fn t2(mut cx: t2::Context) {
        asm::bkpt();
        cx.schedule.t2(cx.scheduled + 200_000.cycles()).unwrap();
        asm::bkpt();

        // 1) your code here to emulate timing behavior of t2
        let start = DWT::get_cycle_count();
        cortex_m::asm::delay(10_000); // 0-10

        // Here R1 is "locked"
        cortex_m::asm::delay(2_000); // 10-12
        cx.resources.R2.lock(|R2| {
            cortex_m::asm::delay(4_000); // 12-16
        });
        cortex_m::asm::delay(4_000); // 16-20
        
        cortex_m::asm::delay(2_000); // 20-22
        // Here R1 is "locked" 
        cortex_m::asm::delay(6_000); // 22-28
        let end = DWT::get_cycle_count();
        let diff = end.wrapping_sub(start);

        // 2) your code here to update T2_MAX_RP and
        // break if deadline missed
        unsafe {
            if diff > T2_MAX_RP {
                T2_MAX_RP = diff;
            }
            if T2_MAX_RP > 200000 {
                panic!("T2 deadline missed");
            }
        }
    }

    // Deadline 50, Inter-arrival 50
    #[inline(never)]
    #[task(schedule = [t3], resources = [R2], priority = 3)]
    fn t3(cx: t3::Context) {
        asm::bkpt();
        cx.schedule.t3(cx.scheduled + 50_000.cycles()).unwrap();
        asm::bkpt();

        // 1) your code here to emulate timing behavior of t3
        let start = DWT::get_cycle_count();
        cortex_m::asm::delay(10_000); // 0-10
        // Here R2 is "locked"
        cortex_m::asm::delay(10_000); // 10-20
        cortex_m::asm::delay(10_000); // 20-30
        let end = DWT::get_cycle_count();
        let diff = end.wrapping_sub(start);

        // 2) your code here to update T3_MAX_RP and
        // break if deadline missed
        unsafe {
            if diff > T3_MAX_RP {
                T3_MAX_RP = diff;
            }
            if T3_MAX_RP > 50000 {
                panic!("T1 deadline missed");
            }
        }
    }

    // RTIC requires that unused interrupts are declared in an extern block when
    // using software tasks; these free interrupts will be used to dispatch the
    // software tasks.
    extern "C" {
        fn EXTI0();
        fn EXTI1();
        fn EXTI2();
    }
};

// !!!! NOTICE !!!!
//
// Use either vscode with the `Cortex Nightly` launch profile,
// or compile with the feature `--features nightly` in order to
// get inlined assembly!
//
// 1) For this assignment you should first generate a task set that
// matches the example task set from `klee_tutorial/srp_analysis/main.rs`.
//
// Assume that each time unit amounts to 1_000 clock cycles, then
// the execution time of `t1` should be 10_000 clock cycles.
//
// So, instead of measuring execution time of an existing application,
// you are to create a task set according to given timing properties.
//
// Do this naively, by just calling `asm::delay(x)`, where x
// amounts to the number of clock cycles to spend.
//
// Commit your repository once your task set is implemented.
//
// 2) Code instrumentation:
// Now its time to see if your scheduling analysis is accurate
// in comparison to a real running system.
//
// First explain in your own words how the `Instance` is
// used to generate a periodic task instance arrivals.
//
// `cx.schedule.t1(cx.scheduled + 100_000.cycles()).unwrap();`
//
// [Your answer here]
// It schedules itself again from it's current scheduled time + another 100k cycles.
// Then it will become a periodic task.
//
// Explain in your own words the difference between:
//
// `cx.schedule.t1(Instance::now() + 100_000.cycles()).unwrap();`
// and
// `cx.schedule.t1(cx.scheduled + 100_000.cycles()).unwrap();`
//
// [Your answer here]
// The first one schedules t1 from the current clock + 100k cycles whereas
// the second one schedules t1 100k cycles relative to the previous scheduled time.
//
// Explain in your own words why we use the latter
// in order to generate a periodic task.
//
// [Your answer here]
// Because then it will be periodic. Using the first method it will not be
// scheduled exactly at 100k intervals.
//
// Hint, look at https://rtic.rs/0.5/book/en/by-example/timer-queue.html
//
// Once you understand how `Instance` is used, document your crate:
// > cargo doc --open
//
// Once you have the documentation open, search for `Instance`
// Hint, you can search docs by pressing S.
//
// Now figure out how to calculate the actual response time.
// If the new response time is larger than the stored response time
// then update it (`T1_MAX_RP`, `T2_MAX_RP`, `T3_MAX_RP` respectively).
// If the response time is larger than the deadline, you should
// hit a `asm::bkpt()`, to indicate that an error occurred.
//
// You will need `unsafe` code to access the global variables.
//
// Explain why this is needed (there is a good reason for it).
//
// [Your answer here]
// It is difficult for the compiler to guarantee that a static mut item
// is handled correctly, due to it pointing to a specific memory address.
//
// Implement this functionality for all tasks.
//
// Commit your repository once you are done with the instrumentation.
//
// 3) Code Testing:
//
// Once the instrumentation code is in place, its finally time
// to test/probe/validate the system.
//
// Make sure that all tasks is initially scheduled from `init`.
//
// You can put WATCHES in vscode for the symbols
// WATCH
//  `T1_MAX_RP`
//  `T2_MAX_RP`
//  `T3_MAX_RP`
// To see them being updated during the test.
//
// The first breakpoint hit should be:
// fn t3(cx: t3::Context) {
//      asm::bkpt();
//
// Check the value of the CYCCNT register.
// (In vscode look under CORTEX PERIPHERALS > DWT > CYCCNT)
//
// Your values may differ slightly but should be in the same
// territory (if not, check your task implementation(s).)
//
// Task Entry Times, Task Nr, Response time Update
//   50240           t3       -
//                            30362
//  100295           t3
//                            30426
//
//  130595           t1
//
// At this point we can ask ourselves a number of
// interesting questions. Try answering in your own words.
//
// 3A) Why is there an offset 50240 (instead of 50000)?
//
// [Your answer here]
//
// 3B) Why is the calculated response time larger than the
// delays you inserted to simulate workload?
//
// [Your answer here]
//
// 3C) Why is the second arrival of `t3` further delayed?
//
// [Your answer here]
// Hint, think about what happens at time 100_000, what tasks
// are set to `arrive` at that point compared to time 50_000.
//
// 3D) What is the scheduled time for task `t1` (130595 is the
// measured time according to CYCYCNT).
//
// [Your answer here]
//
// Why is the measured value much higher than the scheduled time?
//
// [Your answer here]
//
// Now you can continue until you get a first update of `T1_MAX_RP`.
//
// What is the first update of `T1_MAX_RP`?
//
// [Your answer here]
//
// Explain the obtained value in terms of:
// Execution time, blocking and preemptions
// (that occurred for this task instance).
//
// [Your answer here]
//
// Now continue until you get a first timing measurement for `T2_MAX_RP`.
//
// What is the first update of `T2_MAX_RP`?
//
// [Your answer here]
//
// Now continue until you get a second timing measurement for `T1_MAX_RP`.
//
// What is the second update of `T3_MAX_RP`?
//
// [Your answer here]
//
// Now you should have ended up in a deadline miss right!!!!
//
// Why did this happen?
//
// [Your answer here]
//
// Compare that to the result obtained from your analysis tool.
//
// Do they differ, if so why?
//
// [Your answer here]
//
// Commit your repository once you completed this part.
//
// 4) Delay tuning.
//
// So there were some discrepancy between the timing properties
// introduced by the `delay::asm` and the real measurements.
//
// Adjust delays to compensate for the OH to make it fit to
// to the theoretical task set.
//
// In order to do so test each task individually, schedule ony one
// task from `init` at a time.
//
// You may need to insert additional breakpoints to tune the timing.
//
// Once you are convinced that each task now adheres to
// the timing specification you can re-run part 3.
//
// If some task still misses its deadline go back and adjust
// the timing until it just passes.
//
// Commit your tuned task set.
//
// 5) Final remarks and learning outcomes.
//
// This exercise is of course a bit contrived, in the normal case
// you would start out with a real task set and then pass it
// onto analysis.
//
// Essay question:
//
// Reflect in your own words on:
//
// - RTIC and scheduling overhead
// - Coupling in between theoretical model and measurements
// - How would an ideal tool for static analysis of RTIC models look like.
//
// [Your ideas and reflections here]
//
// Commit your thoughts, we will discuss further when we meet.
