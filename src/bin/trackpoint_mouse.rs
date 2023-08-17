#![deny(unsafe_code)]
#![no_main]
#![no_std]

use panic_halt as _;

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true)]
mod app {
    use core::mem::MaybeUninit;

    use stm32f4xx_hal::{
        gpio::{
            alt::otg_fs::{Dm::PA11, Dp::PA12},
            Edge,
            PinState::Low,
        },
        otg_fs::{UsbBus, UsbBusType, USB},
        prelude::*,
        timer::Event,
    };
    use trackpoint_mouse::trackpoint::{
        TrackPoint, RST as TP_RST, SCL as TP_SCL, SDA as TP_SDA, SFACTOR_HIGH as TP_SFACTOR_HIGH,
    };
    use usb_device::{bus::UsbBusAllocator, prelude::*};
    use usbd_hid::{
        descriptor::{generator_prelude::SerializedDescriptor, MouseReport},
        hid_class::HIDClass,
    };

    type HidDev = HIDClass<'static, UsbBusType>;

    #[shared]
    struct Shared {
        hid_ms: HidDev,
        trackpoint: TrackPoint,
        usb_dev: UsbDevice<'static, UsbBus<USB>>,
    }

    #[local]
    struct Local {}

    #[init(local = [
        ep_memory: [u32; 1024] = [0; 1024],
        usb_bus: MaybeUninit<UsbBusAllocator<UsbBusType>> = MaybeUninit::uninit()
    ])]
    fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
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
        let p_rst: TP_RST = gpiob.pb7.into_push_pull_output_in_state(Low).erase();
        let mut p_clk: TP_SCL = gpiob.pb8.into_open_drain_output().erase();
        let p_data: TP_SDA = gpiob.pb9.into_open_drain_output().erase();
        let delay = ctx.core.SYST.delay(&clocks);

        let mut syscfg = ctx.device.SYSCFG.constrain();
        p_clk.make_interrupt_source(&mut syscfg);
        p_clk.enable_interrupt(&mut ctx.device.EXTI);
        p_clk.trigger_on_edge(&mut ctx.device.EXTI, Edge::Falling);

        let mut trackpoint = TrackPoint::new(p_clk, p_data, p_rst, delay);
        trackpoint.reset();
        trackpoint.set_sensitivity_factor(TP_SFACTOR_HIGH);
        // stream mode works
        trackpoint.set_stream_mode();

        let mut timer = ctx.device.TIM2.counter_hz(&clocks);
        timer.start(1.kHz()).unwrap();
        timer.listen(Event::Update);

        let usb_bus = ctx
            .local
            .usb_bus
            .write(UsbBus::new(usb, ctx.local.ep_memory));

        let hid_ms = HIDClass::new(usb_bus, MouseReport::desc(), 10);

        let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x2023, 0x0610))
            .manufacturer("Custom")
            .product("Trackpoint Mouse")
            .serial_number("20221010")
            .device_class(0)
            .build();
        (
            Shared {
                hid_ms,
                trackpoint,
                usb_dev,
            },
            Local {},
            init::Monotonics(),
        )
    }

    #[task(binds=EXTI9_5, shared = [trackpoint])]
    fn rx_trackpoint_data(mut ctx: rx_trackpoint_data::Context) {
        ctx.shared.trackpoint.lock(|tp| {
            tp.cache_stream_data_bit(); // must read data before clear interrupt
            tp.scl.clear_interrupt_pending_bit();
        })
    }

    #[task(binds=TIM2, shared = [hid_ms, trackpoint])]
    fn tx_mouse_report(ctx: tx_mouse_report::Context) {
        (ctx.shared.hid_ms, ctx.shared.trackpoint).lock(|hid, tp| {
            if tp.data_available {
                hid.push_input(&MouseReport {
                    x: tp.data.x,
                    y: -tp.data.y,
                    buttons: tp.data.state % 16 % 7, // BTN1: 1, BTN2: 2, BTN3: 4
                    wheel: 0,
                    pan: 0,
                })
                .ok();
            }
        });
    }

    #[task(binds=OTG_FS, shared = [hid_ms, usb_dev])]
    fn on_usb(ctx: on_usb::Context) {
        (ctx.shared.usb_dev, ctx.shared.hid_ms).lock(|usb_dev, hid| if usb_dev.poll(&mut [hid]) {});
    }
}
