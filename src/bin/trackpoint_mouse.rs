#![deny(unsafe_code)]
#![no_main]
#![no_std]

use panic_halt as _;

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true)]
mod app {
    use core::mem::MaybeUninit;
    use frunk::HList;

    use stm32f4xx_hal::{
        gpio::alt::otg_fs::{Dm::PA11, Dp::PA12},
        otg_fs::{UsbBus, UsbBusType, USB},
        prelude::*,
        timer::Event,
    };
    use trackpoint_mouse::trackpoint::TrackPoint;
    use usb_device::{bus::UsbBusAllocator, prelude::*};
    use usbd_human_interface_device::{
        device::mouse::{BootMouse, BootMouseConfig, BootMouseReport},
        prelude::*,
    };

    const TP_P: char = 'B';
    const TP_CLK: u8 = 8;
    const TP_DATA: u8 = 9;
    const TP_RST: u8 = 7;

    #[shared]
    struct Shared {
        mixed_hid: UsbHidClass<'static, UsbBusType, HList!(BootMouse<'static, UsbBusType>,)>,
        trackpoint: TrackPoint<TP_P, TP_CLK, TP_DATA, TP_RST>,
        usb_dev: UsbDevice<'static, UsbBus<USB>>,
    }

    #[local]
    struct Local {}

    #[init(local = [
        ep_memory: [u32; 1024] = [0; 1024],
        usb_bus: MaybeUninit<UsbBusAllocator<UsbBusType>> = MaybeUninit::uninit()
    ])]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let rcc = ctx.device.RCC.constrain();
        let clocks = rcc
            .cfgr
            .use_hse(25.MHz())
            .sysclk(84.MHz())
            .require_pll48clk()
            .freeze();

        let gpioa = ctx.device.GPIOA.split();
        let usb = USB {
            usb_global: ctx.device.OTG_FS_GLOBAL,
            usb_device: ctx.device.OTG_FS_DEVICE,
            usb_pwrclk: ctx.device.OTG_FS_PWRCLK,
            pin_dm: PA11(gpioa.pa11.into_alternate()),
            pin_dp: PA12(gpioa.pa12.into_alternate()),
            hclk: clocks.hclk(),
        };

        let gpiob = ctx.device.GPIOB.split();
        let p_rst = gpiob.pb7.into_push_pull_output();
        let p_clk = gpiob.pb8.into_dynamic();
        let p_data = gpiob.pb9.into_dynamic();
        let delay = ctx.core.SYST.delay(&clocks);

        let mut trackpoint = TrackPoint::new(p_clk, p_data, p_rst, delay);
        trackpoint.reset();
        trackpoint.set_sensitivity_factor(0xCC);
        trackpoint.set_stream_mode();

        let mut timer = ctx.device.TIM2.counter_hz(&clocks);
        timer.start(1.kHz()).unwrap();
        timer.listen(Event::Update);

        let usb_bus = ctx
            .local
            .usb_bus
            .write(UsbBus::new(usb, ctx.local.ep_memory));

        let mixed_hid = UsbHidClassBuilder::new()
            .add_device(BootMouseConfig::default())
            .build(usb_bus);

        let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x2023, 0x0610))
            .manufacturer("Custom KeyBoard Maker")
            .product("Trackpoint Mouse")
            .serial_number("20221010")
            .device_class(0)
            .build();
        (
            Shared {
                mixed_hid,
                trackpoint,
                usb_dev,
            },
            Local {},
            init::Monotonics(),
        )
    }

    #[task(binds=TIM2, shared = [mixed_hid, trackpoint], local=[])]
    fn tp_data(ctx: tp_data::Context) {
        (ctx.shared.mixed_hid, ctx.shared.trackpoint).lock(|mixed_hid, trackpoint| {
            let (state, tx, ty) = (trackpoint.read(), trackpoint.read(), trackpoint.read());
            let report = BootMouseReport {
                x: tx as i8,
                y: -(ty as i8),
                buttons: state & 7u8,
            };

            let mouse = mixed_hid.device();
            match mouse.write_report(&report) {
                Err(UsbHidError::WouldBlock) => {}
                Ok(_) => {}
                Err(e) => {
                    core::panic!("Failed to write mouse report: {:?}", e)
                }
            }
        });
    }

    #[task(binds=OTG_FS, shared = [mixed_hid, usb_dev])]
    fn on_usb(ctx: on_usb::Context) {
        (ctx.shared.usb_dev, ctx.shared.mixed_hid).lock(
            |usb_dev, mixed_hid| {
                if usb_dev.poll(&mut [mixed_hid]) {}
            },
        );
    }
}
