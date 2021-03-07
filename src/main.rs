#![no_std]
#![no_main]

mod webusb;

extern crate panic_reset;

use crate::webusb::WebUSB;
use core::convert::Infallible;
use cortex_m_rt::entry;
use stm32_device_signature::device_id_hex;
use stm32_usbd::UsbBus;
use stm32f0xx_hal::{
    gpio::{Output, Pin, PushPull},
    prelude::*,
    serial::Serial,
    stm32,
};
use usb_device::prelude::*;
use usbd_serial::SerialPort;

#[entry]
fn main() -> ! {
    let mut dp = stm32::Peripherals::take().unwrap();

    dp.RCC.apb2enr.modify(|_, w| w.syscfgen().set_bit());
    dp.SYSCFG.cfgr1.modify(|_, w| w.pa11_pa12_rmp().remapped());

    let mut rcc = dp
        .RCC
        .configure()
        .hsi48()
        .enable_crs(dp.CRS)
        .sysclk(48.mhz())
        .pclk(24.mhz())
        .freeze(&mut dp.FLASH);

    let gpioa = dp.GPIOA.split(&mut rcc);
    let gpiob = dp.GPIOB.split(&mut rcc);

    let (usb_dm, usb_dp, uart_tx, uart_rx, mut esp_en, mut esp_gpio0, mut led) =
        cortex_m::interrupt::free(|cs| {
            (
                gpioa.pa11,
                gpioa.pa12,
                gpioa.pa2.into_alternate_af1(cs),
                gpioa.pa3.into_alternate_af1(cs),
                gpioa.pa1.into_push_pull_output(cs).downgrade(),
                gpioa.pa4.into_push_pull_output(cs).downgrade(),
                gpiob.pb1.into_push_pull_output(cs).downgrade(),
            )
        });

    let _ = esp_en.set_high();
    let _ = esp_gpio0.set_high();
    let _ = led.set_high();

    let usb_bus = UsbBus::new(dp.USB, (usb_dm, usb_dp));

    let mut usb_serial = SerialPort::new(&usb_bus);
    let mut webusb = WebUSB::new(&usb_bus);

    let mut uart = Serial::usart2(dp.USART2, (uart_tx, uart_rx), 115_200.bps(), &mut rcc);

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .manufacturer("Electromagnetic Field")
        .product("TiLDA MkV")
        .serial_number(device_id_hex())
        .max_power(500)
        .build();

    loop {
        if usb_dev.poll(&mut [&mut usb_serial, &mut webusb]) {
            led.set_low().unwrap();
            let mut buf = [0u8; 64];
            match usb_serial.read(&mut buf) {
                Ok(count) if count > 0 => {
                    let mut offset = 0;
                    while offset < count {
                        if let Ok(()) = uart.write(buf[offset]) {
                            offset += 1;
                        }
                    }
                }
                _ => {}
            }

            match webusb.read(&mut buf) {
                Ok(count) if count > 0 => {
                    let mut offset = 0;
                    while offset < count {
                        if let Ok(()) = uart.write(buf[offset]) {
                            offset += 1;
                        }
                    }
                }
                _ => {}
            }

            // Set the ESP32 boot pins based on the RTS/DTR pins.
            // These are inverted because the USB flags are true when asserted where as the serial
            // lines are low when asserted.
            let _ = set_pins(
                !(usb_serial.dtr() || webusb.dtr()),
                !(usb_serial.rts() || webusb.rts()),
                &mut esp_en,
                &mut esp_gpio0,
            );
            led.set_high().unwrap();
        }

        if usb_dev.state() == UsbDeviceState::Configured {
            // USB device is active.
            while let Ok(byte) = uart.read() {
                led.set_low().unwrap();
                // Write input from UART to both USB endpoints, ignoring errors.
                let _ = usb_serial.write(&[byte]);
                let _ = webusb.write(&[byte]);
            }
            led.set_high().unwrap();
        }
    }
}

/// Set the ESP boot control pins based on serial DTR and RTS pins.
/// This emulates the transistor logic implemented on ESP32 dev boards to ignore DTR and RTS being
/// asserted simultaneously.
fn set_pins(
    dtr: bool,
    rts: bool,
    esp_en: &mut Pin<Output<PushPull>>,
    esp_gpio0: &mut Pin<Output<PushPull>>,
) -> Result<(), Infallible> {
    // https://github.com/espressif/esptool/wiki/ESP32-Boot-Mode-Selection#automatic-bootloader
    // DTR RTS| EN  IO0
    // 1   1  |  1   1
    // 0   0  |  1   1
    // 1   0  |  0   1
    // 0   1  |  1   0

    if !rts && !dtr {
        esp_gpio0.set_high()?;
        esp_en.set_high()?;
    } else {
        if dtr {
            esp_gpio0.set_high()?;
        } else {
            esp_gpio0.set_low()?;
        }

        if rts {
            esp_en.set_high()?;
        } else {
            esp_en.set_low()?;
        }
    }
    Ok(())
}
