//! Example of using CAN.
#![no_std]
#![no_main]

use panic_semihosting as _;

use stm32f3xx_hal as hal;

use cortex_m_rt::entry;

use cortex_m::asm;
use hal::prelude::*;
use hal::stm32;
use hal::watchdog::IndependentWatchDog;

use embedded_hal::Direction;
use hal::can::{Can, CanFilter, CanFrame, CanId, Filter, Frame, Receiver, Transmitter};
use nb::block;
use stm32f3xx_hal::qei::EncoderMode;

// Each "node" needs a different ID, we set up a filter too look for messages to this ID
// Max value is 8 because we use a 3 bit mask in the filter
const ID: u16 = 0b100;

#[entry]
fn main() -> ! {
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

    let mut led0 = gpiob
        .pb15
        .into_push_pull_output(&mut gpiob.moder, &mut gpiob.otyper);
    led0.set_high().unwrap();

    // Watchdog makes sure this gets restarted periodically if nothing happens
    let mut iwdg = IndependentWatchDog::new(dp.IWDG);
    iwdg.stop_on_debug(&dp.DBGMCU, true);
    iwdg.start(100.ms());

    let mut i: u16 = 1;
    loop {
        let count = qei.count();
        if qei.direction() == Direction::Upcounting && count > (i * 1000) {
            led0.toggle();
            i += 1;
        } else if qei.direction() == Direction::Downcounting && count < (i * 1000) {
            led0.toggle();
            i -= 1;
        }

        if count != 1 {
            iwdg.feed();
        }

        asm::delay(1_000);
    }
}
