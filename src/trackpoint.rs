#![deny(unsafe_code)]

use panic_halt as _;

use stm32f4xx_hal::{
    gpio::{DynamicPin, Output, Pin, PinState::Low, PushPull},
    prelude::*,
    timer::delay::SysDelay,
};

pub struct DataReport {
    pub state: u8,
    pub x: i8,
    pub y: i8,
}

const CC_READ_DATA: u8 = 0xEB;
const CC_SNSTVTY: u8 = 0x4A;
const CC_RAM: u8 = 0xE2;
const CC_SET: u8 = 0x81;
const CC_ENABLE: u8 = 0xF4;
const CC_STREAM_MODE: u8 = 0xEA;

pub struct TrackPoint<const P: char, const CLK: u8, const DATA: u8, const RST: u8> {
    pub scl: DynamicPin<P, CLK>,
    pub sda: DynamicPin<P, DATA>,
    pub rst: Pin<P, RST, Output<PushPull>>,
    pub delay: SysDelay,
}

impl<const P: char, const CLK: u8, const DATA: u8, const RST: u8> TrackPoint<P, CLK, DATA, RST> {
    pub fn new(
        scl: DynamicPin<P, CLK>,
        sda: DynamicPin<P, DATA>,
        rst: Pin<P, RST, Output<PushPull>>,
        delay: SysDelay,
    ) -> Self {
        Self {
            scl,
            sda,
            rst,
            delay,
        }
    }

    pub fn query_data_report(&mut self) -> DataReport {
        self.write(CC_READ_DATA);
        self.read();
        DataReport {
            state: self.read(),
            x: self.read() as i8,
            y: self.read() as i8,
        }
    }

    pub fn is_scl_hi(&mut self) -> bool {
        self.scl.is_high().unwrap()
    }

    pub fn is_scl_lo(&mut self) -> bool {
        self.scl.is_low().unwrap()
    }

    pub fn is_sda_hi(&mut self) -> bool {
        self.sda.is_high().unwrap()
    }

    pub fn is_sda_lo(&mut self) -> bool {
        self.sda.is_low().unwrap()
    }

    pub fn set_scl_hi(&mut self) {
        self.scl.make_pull_up_input();
    }

    pub fn set_scl_lo(&mut self) {
        self.scl.make_push_pull_output_in_state(Low);
    }

    pub fn set_sda_hi(&mut self) {
        self.sda.make_pull_up_input()
    }

    pub fn set_sda_lo(&mut self) {
        self.sda.make_push_pull_output_in_state(Low);
    }

    pub fn reset(&mut self) {
        self.rst.set_high();
        self.delay.delay_ms(1000_u16);
        self.rst.set_low();
    }

    pub fn set_sensitivity_factor(&mut self, sensitivity_factor: u8) {
        self.write_to_ram_location(CC_SNSTVTY, sensitivity_factor);
    }

    pub fn write_to_ram_location(&mut self, location: u8, value: u8) {
        self.write(CC_RAM);
        self.read();

        self.write(CC_SET);
        self.read();

        self.write(location);
        self.read();

        self.write(value);
        self.read();
    }

    pub fn set_stream_mode(&mut self) {
        self.write(CC_STREAM_MODE);
        self.read();
        self.write(CC_ENABLE);
        self.read();

        self.set_scl_hi();
        self.set_sda_hi();
    }

    pub fn read(&mut self) -> u8 {
        let mut data = 0x00;
        let mut bit = 0x01;
        self.set_scl_hi();
        self.set_sda_hi();
        self.delay.delay_us(50_u8);
        while self.is_scl_hi() {}
        self.delay.delay_us(5_u8);
        while self.is_scl_lo() {}
        for _ in 0..8 {
            while self.is_scl_hi() {}
            if self.is_sda_hi() {
                data |= bit;
            }
            while self.is_scl_lo() {}
            bit <<= 1;
        }
        while self.is_scl_hi() {}

        while self.is_scl_lo() {}
        while self.is_scl_hi() {}
        while self.is_scl_lo() {}
        self.set_scl_lo();
        data
    }

    /* write a uint8_t to the trackpoint */
    pub fn write(&mut self, mut data: u8) {
        let mut parity: u8 = 1;
        self.set_sda_hi();
        self.set_scl_hi();
        self.delay.delay_us(300_u16);
        self.set_scl_lo();
        self.delay.delay_us(300_u16);
        self.set_sda_lo();
        self.delay.delay_us(10_u8);
        self.set_scl_hi();

        /* wait for trackpoint to take control of clock */
        while self.is_scl_hi() {}

        for _ in 0..8 {
            if data & 0x01 > 0 {
                self.set_sda_hi();
            } else {
                self.set_sda_lo();
            }
            while self.is_scl_lo() {}
            while self.is_scl_hi() {}
            parity ^= data & 0x01;
            data >>= 1;
        }
        if parity > 0 {
            self.set_sda_hi();
        } else {
            self.set_sda_lo();
        }
        while self.is_scl_lo() {}
        while self.is_scl_hi() {}
        self.set_sda_hi();
        self.delay.delay_us(50_u8);
        while self.is_scl_hi() {}
        while self.is_scl_lo() || self.is_sda_lo() {}
        self.set_scl_lo();
    }
}
