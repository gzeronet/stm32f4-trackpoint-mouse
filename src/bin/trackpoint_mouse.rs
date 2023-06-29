#![deny(unsafe_code)]
#![no_main]
#![no_std]

use panic_halt as _;

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true)]
mod app {
    use core::mem::MaybeUninit;
    use stm32f4xx_hal::{
        gpio::alt::otg_fs::{Dm::PA11, Dp::PA12},
        otg_fs::{UsbBus, UsbBusType, USB},
        prelude::*,
        timer::Event,
    };
    use trackpoint_mouse::trackpoint::TrackPoint;
    use usb_device::{bus::UsbBusAllocator, prelude::*};
    use usbd_hid::{
        descriptor::{generator_prelude::SerializedDescriptor, MouseReport},
        hid_class::HIDClass,
    };

    #[shared]
    struct Shared {
        hid: HIDClass<'static, UsbBusType>,
        trackpoint: TrackPoint,
    }

    #[local]
    struct Local {
        usb_dev: UsbDevice<'static, UsbBus<USB>>,
    }

    #[init(local = [ep_memory: [u32; 1024] = [0; 1024], usb_bus: MaybeUninit<UsbBusAllocator<UsbBusType>> = MaybeUninit::uninit()])]
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

        let usb_bus = ctx.local.usb_bus;
        let usb_bus = usb_bus.write(UsbBus::new(usb, ctx.local.ep_memory));

        let hid = HIDClass::new(usb_bus, MouseReport::desc(), 1);
        let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x2023, 0x0610))
            .manufacturer("Custom KeyBoard Maker")
            .product("Trackpoint Mouse")
            .serial_number("20221010")
            .device_class(0)
            .build();

        (
            Shared { hid, trackpoint },
            Local { usb_dev },
            init::Monotonics(),
        )
    }

    #[task(binds=TIM2, shared = [hid, trackpoint])]
    fn tp_data(mut ctx: tp_data::Context) {
        ctx.shared.trackpoint.lock(|trackpoint| {
            let (state, tx, ty) = (trackpoint.read(), trackpoint.read(), trackpoint.read());
            let report = MouseReport {
                x: tx as i8,
                y: -(ty as i8),
                buttons: state & 7u8,
                wheel: 0,
                pan: 0,
            };
            ctx.shared.hid.lock(|hid| hid.push_input(&report).ok());
        });
    }

    #[task(binds=OTG_FS, shared = [hid], local=[ usb_dev])]
    fn on_usb(mut ctx: on_usb::Context) {
        let usb_dev = ctx.local.usb_dev;
        ctx.shared.hid.lock(|hid| if !usb_dev.poll(&mut [hid]) {});
    }
}
