use crate::rcc::Clocks;
/// Quadrature Encoder Interface
use embedded_hal::{Direction, Qei};

use crate::gpio::{gpioa, Input, AF2};
use embedded_hal::digital::InputPin;
use stm32f3::stm32f302::TIM3;

pub enum EncoderMode {
    Edge1,
    Edge2,
    BothEdges,
}

pub struct QeiTimer<TIM, CH1, CH2> {
    //clocks: Clocks,
    tim: TIM,
    ch1: CH1,
    ch2: CH2,
}

impl Qei for QeiTimer<TIM3, gpioa::PA6<AF2>, gpioa::PA7<AF2>> {
    type Count = u16;

    fn count(&self) -> Self::Count {
        //cortex_m::asm::bkpt();
        self.tim.cnt.read().cnt().bits()
    }

    fn direction(&self) -> Direction {
        if self.tim.cr1.read().dir().is_up() {
            Direction::Upcounting
        } else {
            Direction::Downcounting
        }
    }
}

impl QeiTimer<TIM3, gpioa::PA6<AF2>, gpioa::PA7<AF2>> {
    pub fn new(
        tim: TIM3,
        apb: &mut crate::rcc::APB1,
        mode: EncoderMode,
        arr: u16,
        ch1: gpioa::PA6<AF2>,
        ch2: gpioa::PA7<AF2>,
    ) -> QeiTimer<TIM3, gpioa::PA6<AF2>, gpioa::PA7<AF2>> {
        apb.enr().modify(|_, w| w.tim3en().enabled());
        apb.rstr().modify(|_, w| w.tim3rst().reset());
        apb.rstr().modify(|_, w| w.tim3rst().clear_bit());

        tim.smcr.modify(|_, w| match mode {
            EncoderMode::Edge1 => w.sms().encoder_mode_1(),
            EncoderMode::Edge2 => w.sms().encoder_mode_2(),
            EncoderMode::BothEdges => w.sms().encoder_mode_3(),
        });

        tim.ccer.modify(|_, w| {
            w.cc1p().clear_bit();
            w.cc2p().clear_bit();
            w.cc1np().clear_bit();
            w.cc2np().clear_bit()
        });

        tim.arr.modify(|_, w| unsafe { w.bits(arr as u32) });

        tim.cr1.modify(|_, w| w.cen().enabled());

        QeiTimer { tim, ch1, ch2 }
    }

    pub fn reset(&mut self) {
        self.tim.cnt.modify(|_, w| unsafe { w.bits(0u32) });
    }

    pub fn release(self) -> (TIM3, gpioa::PA6<AF2>, gpioa::PA7<AF2>) {
        return (self.tim, self.ch1, self.ch2);
    }
}
