#![deny(unsafe_code)]
// #![allow(warnings)]
#![allow(clippy::empty_loop)]

use panic_halt as _;

use stm32f4xx_hal::{
    gpio::{self, PinState::Low},
    prelude::*,
    timer::delay::SysDelay,
};

pub struct DataReport {
    pub state: u8,
    pub x: i8,
    pub y: i8,
}

const P: char = 'B';
const N_CLK: u8 = 8;
const N_DATA: u8 = 9;

pub struct TrackPoint {
    pub scl: gpio::DynamicPin<P, N_CLK>,
    pub sda: gpio::DynamicPin<P, N_DATA>,
    pub rst: gpio::gpiob::PB7<gpio::Output<gpio::PushPull>>,
    pub delay: SysDelay,
}

impl TrackPoint {
    pub fn new(
        scl: gpio::DynamicPin<P, N_CLK>,
        sda: gpio::DynamicPin<P, N_DATA>,
        rst: gpio::gpiob::PB7<gpio::Output<gpio::PushPull>>,
        delay: SysDelay,
    ) -> Self {
        Self {
            scl,
            sda,
            rst,
            delay,
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
        self.delay.delay_ms(2000_u16);
        self.rst.set_low();
    }

    pub fn set_sensitivity_factor(&mut self, sensitivity_factor: u8) {
        self.write_to_ram_location(0x4a, sensitivity_factor);
    }

    pub fn write_to_ram_location(&mut self, location: u8, value: u8) {
        // self.write(0xe2);
        self.write(0xe2);

        // self.read(); // ACK
        self.read();

        // write(0x81);
        self.write(0x81);
        // read(); // ACK
        self.read();

        // let location = 0x4a;
        // write(location);
        self.write(location);
        // read(); // ACK
        self.read();

        // let value = 0x99;
        // write(value);
        self.write(value);
        // read(); // ACK
        self.read();
    }

    pub fn set_stream_mode(&mut self) {
        // write(0xea);
        self.write(0xea);
        self.read();
        // write(0xf4); //enable report
        self.write(0xf4);
        self.read();

        // put mouse into idle mode, ready to send
        // gohi(_clkPin);
        self.set_scl_hi();
        // gohi(_dataPin);
        self.set_sda_hi();
    }

    pub fn read(&mut self) -> u8 {
        // uint8_t data = 0x00;
        let mut data = 0x00;
        // uint8_t bit = 0x01;
        let mut bit = 0x01;
        // start clock
        // gohi(_clkPin);
        self.set_scl_hi();
        // gohi(_dataPin);
        self.set_sda_hi();
        // delayMicroseconds(50);
        self.delay.delay_us(50_u8);

        // while (digitalRead(_clkPin) == HIGH) ;
        while self.is_scl_hi() {} // scl 1st fall
                                  // delayMicroseconds(5);	// not sure why.
        self.delay.delay_us(5_u8);
        // while (digitalRead(_clkPin) == LOW) ;	// eat start bit
        while self.is_scl_lo() {}
        // for (i=0; i < 8; i++)
        for _ in 0..8 {
            // while (digitalRead(_clkPin) == HIGH) ;
            while self.is_scl_hi() {}
            // if (digitalRead(_dataPin) == HIGH)
            if self.is_sda_hi() {
                // data = data | bit;
                data |= bit;
            }
            // while (digitalRead(_clkPin) == LOW) ;
            while self.is_scl_lo() {}
            // bit = bit << 1;
            bit <<= 1;
        }
        // eat parity bit, ignore it.
        // self.delay.delay_us(1_u8);
        // while (digitalRead(_clkPin) == HIGH) ;
        while self.is_scl_hi() {}

        // if self.is_sda_hi() {} // parity
        // self.delay.delay_us(1_u8);
        // while (digitalRead(_clkPin) == LOW) ;
        while self.is_scl_lo() {}
        // eat stop bit
        // self.delay.delay_us(1_u8);
        // while (digitalRead(_clkPin) == HIGH) ;
        while self.is_scl_hi() {}
        // self.delay.delay_us(1_u8);
        // while (digitalRead(_clkPin) == LOW)
        while self.is_scl_lo() {}
        // golo(_clkPin);	// hold incoming data
        self.set_scl_lo();
        // return data
        data
    }

    /* write a uint8_t to the PS2 device */
    pub fn write(&mut self, mut data: u8) {
        let mut parity: u8 = 1;
        // gohi(_clkPin);
        self.set_scl_hi();
        // gohi(_dataPin);
        self.set_sda_hi();
        // delayMicroseconds(300);
        self.delay.delay_us(300_u16);
        // golo(_clkPin);
        self.set_scl_lo();
        // delayMicroseconds(300);
        self.delay.delay_us(300_u16);
        // golo(_dataPin);
        self.set_sda_lo();
        // delayMicroseconds(10);
        self.delay.delay_us(10_u8);
        // gohi(_clkPin);	// start bit
        self.set_scl_hi();
        // MADEUP
        // self.delay.delay_us(10_u32);

        /* wait for device to take control of clock */
        // while (digitalRead(_clkPin) == HIGH)
        while self.is_scl_hi() {}
        // ;	// this loop intentionally left blank

        // clear to send data
        // for (i=0; i < 8; i++)
        for _ in 0..8 {
            // if (data & 0x01)
            if data & 0x01 > 0 {
                // gohi(_dataPin);
                self.set_sda_hi();
            } else {
                // golo(_dataPin);
                self.set_sda_lo();
            }
            // wait for clock
            //    while (digitalRead(_clkPin) == LOW)
            while self.is_scl_lo() {}
            // while (digitalRead(_clkPin) == HIGH)
            while self.is_scl_hi() {}
            // parity = parity ^ (data & 0x01);
            parity ^= data & 0x01;
            // data = data >> 1;
            data >>= 1;
        }
        // parity bit
        //    if (parity)
        if parity > 0 {
            // gohi(_dataPin);
            self.set_sda_hi();
        } else {
            // golo(_dataPin);
            self.set_sda_lo();
        }
        // clock cycle - like ack.
        // while (digitalRead(_clkPin) == LOW)
        while self.is_scl_lo() {}
        // while (digitalRead(_clkPin) == HIGH)
        while self.is_scl_hi() {}
        // stop bit
        // gohi(_dataPin);
        self.set_sda_hi();
        // delayMicroseconds(50);
        self.delay.delay_us(50_u8);
        // while (digitalRead(_clkPin) == HIGH)
        while self.is_scl_hi() {}
        // mode switch
        // while ((digitalRead(_clkPin) == LOW) || (digitalRead(_dataPin) == LOW))
        while self.is_scl_lo() || self.is_sda_lo() {}
        // hold up incoming data
        // golo(_clkPin);
        self.set_scl_lo();
    }
}
