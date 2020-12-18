//! Example of using CAN.
#![no_std]
#![no_main]

use panic_semihosting as _;

use stm32f3xx_hal as hal;

use cortex_m_rt::entry;

use cortex_m::{asm, interrupt::Mutex};
use hal::prelude::*;
use hal::stm32;
use hal::stm32::{interrupt, Interrupt, TIM3};
use hal::watchdog::IndependentWatchDog;

use core::borrow::Borrow;
use core::cell::RefCell;
use core::ops::{Deref, DerefMut};
use embedded_hal::Direction;
use hal::can::{Can, CanFilter, CanFrame, CanId, Filter, Frame, Receiver, Transmitter};
use nb::block;
use stm32f3::stm32f302::EXTI;
use stm32f3xx_hal::gpio::gpioa::{PA6, PA7};
use stm32f3xx_hal::gpio::AF2;
use stm32f3xx_hal::qei::{EncoderMode, QeiTimer};

// Each "node" needs a different ID, we set up a filter too look for messages to this ID
// Max value is 8 because we use a 3 bit mask in the filter
const ID: u16 = 0b100;

static QEI: Mutex<RefCell<Option<QeiTimer<TIM4, PB6<AF2>, PB7<AF2>>>>> =
    Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    let cp = stm32::CorePeripherals::take().unwrap();
    let dp = stm32::Peripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();
    let mut gpiob = dp.GPIOB.split(&mut rcc.ahb);
    let mut gpioa = dp.GPIOA.split(&mut rcc.ahb);

    let _clocks = rcc
        .cfgr
        .use_hse(32.mhz())
        .sysclk(32.mhz())
        .pclk1(16.mhz())
        .pclk2(16.mhz())
        .freeze(&mut flash.acr);

    rcc.apb2.enr().modify(|_, w| w.syscfgen().set_bit());

    let ch1 = gpioa.pa6.into_af2(&mut gpioa.moder, &mut gpioa.afrl);
    let ch2 = gpioa.pa7.into_af2(&mut gpioa.moder, &mut gpioa.afrl);

    let qei_init = hal::qei::QeiTimer::new(
        dp.TIM3,
        &mut rcc.apb1,
        EncoderMode::BothEdges,
        4096,
        ch1,
        ch2,
    );

    cortex_m::interrupt::free(move |cs| QEI.borrow(cs).replace(Some(qei_init)));

    let mut led0 = gpiob
        .pb15
        .into_push_pull_output(&mut gpiob.moder, &mut gpiob.otyper);
    led0.set_high().unwrap();

    let mut led1 = gpiob
        .pb14
        .into_push_pull_output(&mut gpiob.moder, &mut gpiob.otyper);
    led1.set_high().unwrap();

    let _index = gpiob.pb0.into_pull_down_input();

    unsafe {
        dp.SYSCFG.exticr1.modify(|_, w| w.exti0().bits(0b001));
    }
    dp.EXTI.imr1.modify(|_, w| w.mr0().set_bit());
    dp.EXTI.rtsr1.modify(|_, w| w.tr0().set_bit());
    let mut nvic = cp.NVIC;
    unsafe {
        nvic.set_priority(Interrupt::EXTI0, 1);
        cortex_m::peripheral::NVIC::unmask(Interrupt::EXTI0);
    }
    cortex_m::peripheral::NVIC::unpend(Interrupt::EXTI0);

    let mut i: u16 = 1;
    loop {
        let (count, dir) = cortex_m::interrupt::free(move |cs| {
            if let &Some(ref qei) = QEI.borrow(cs).borrow().deref() {
                (qei.count(), qei.direction())
            } else {
                (0, Direction::Upcounting)
            }
        });

        assert!(count <= 4096, "Count OOB {}", count);

        if dir == Direction::Upcounting {
            led1.set_high().unwrap();
        } else {
            led1.set_low().unwrap();
        }

        if count % 300 == 0 {
            led0.toggle();
        }
    }
}

#[interrupt]
fn EXTI0() {
    cortex_m::interrupt::free(|cs| {
        //cortex_m::peripheral::NVIC::unpend(Interrupt::EXTI0);
        unsafe {
            (*EXTI::ptr()).pr1.modify(|_, w| w.pr0().set_bit());
        }

        if let &mut Some(ref mut qei) = QEI.borrow(cs).borrow_mut().deref_mut() {
            qei.reset();
        }
    });
}
