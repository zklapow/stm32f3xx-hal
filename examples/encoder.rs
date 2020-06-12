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

use core::cell::RefCell;
use core::ops::DerefMut;
use embedded_hal::Direction;
use hal::can::{Can, CanFilter, CanFrame, CanId, Filter, Frame, Receiver, Transmitter};
use nb::block;
use stm32f3xx_hal::gpio::gpioa::{PA6, PA7};
use stm32f3xx_hal::gpio::AF2;
use stm32f3xx_hal::qei::{EncoderMode, QeiTimer};

// Each "node" needs a different ID, we set up a filter too look for messages to this ID
// Max value is 8 because we use a 3 bit mask in the filter
const ID: u16 = 0b100;

static QEI: Mutex<RefCell<Option<QeiTimer<TIM3, PA6<AF2>, PA7<AF2>>>>> =
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

    let qei = hal::qei::QeiTimer::new(
        dp.TIM3,
        &mut rcc.apb1,
        EncoderMode::BothEdges,
        10000,
        ch1,
        ch2,
    );

    cortex_m::interrupt::free(move |cs| {
        *QEI.borrow(cs).borrow_mut() = Some(qei);
    });

    let mut led0 = gpiob
        .pb15
        .into_push_pull_output(&mut gpiob.moder, &mut gpiob.otyper);
    led0.set_high().unwrap();

    // Watchdog makes sure this gets restarted periodically if nothing happens
    let mut iwdg = IndependentWatchDog::new(dp.IWDG);
    iwdg.stop_on_debug(&dp.DBGMCU, true);
    iwdg.start(100.ms());

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
        let mut count = 0;
        let mut dir: Direction = Direction::Upcounting;
        cortex_m::interrupt::free(move |cs| {
            if let &mut Some(ref mut qei) = QEI.borrow(cs).borrow_mut().deref_mut() {
                count = qei.count();
                dir = qei.direction();
            }
        });

        if dir == Direction::Upcounting && count > (i * 1000) {
            led0.toggle();
            i += 1;
        } else if dir == Direction::Downcounting && count < (i * 1000) {
            led0.toggle();
            i -= 1;
        }

        iwdg.feed();

        asm::delay(1_000);
    }
}

#[interrupt]
fn EXTI0() {
    cortex_m::interrupt::free(|cs| {
        if let &mut Some(ref mut qei) = QEI.borrow(cs).borrow_mut().deref_mut() {
            qei.reset();
        }
    });
}
