#![no_std]
#![no_main]

mod webusb;

extern crate panic_semihosting;

use crate::webusb::WebUSB;
use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use stm32_device_signature::device_id_hex;
use stm32_usbd::UsbBus;
use stm32f0xx_hal::{prelude::*, serial::Serial, stm32};
use usb_device::prelude::*;
use usbd_serial::SerialPort;

#[entry]
fn main() -> ! {
    hprintln!("START").unwrap();
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

    let (usb_dm, usb_dp, uart_tx, uart_rx, mut esp_en, mut esp_gpio0) = cortex_m::interrupt::free(|cs| {
        (
            gpioa.pa11,
            gpioa.pa12,
            gpioa.pa2.into_alternate_af1(cs),
            gpioa.pa3.into_alternate_af1(cs),
            gpioa.pa1.into_push_pull_output(cs),
            gpioa.pa4.into_push_pull_output(cs),
        )
    });
    
    esp_en.set_high().unwrap();
    esp_gpio0.set_high().unwrap();

    let usb_bus = UsbBus::new(dp.USB, (usb_dm, usb_dp));

    let mut serial = SerialPort::new(&usb_bus);
    let mut webusb = WebUSB::new(&usb_bus);

    let mut uart = Serial::usart2(dp.USART2, (uart_tx, uart_rx), 115_200.bps(), &mut rcc);

    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .manufacturer("Electromagnetic Field")
        .product("TiLDA MkV")
        .serial_number(device_id_hex())
        .max_power(500)
        .build();

    loop {
        if usb_dev.poll(&mut [&mut serial, &mut webusb]) {
            let mut buf = [0u8; 64];
            match serial.read(&mut buf) {
                Ok(count) if count > 0 => {
                    for byte in &buf[0..count] {
                        if let Ok(()) = uart.write(*byte) {}
                    }
                }
                _ => {}
            }

            match webusb.read(&mut buf) {
                Ok(count) if count > 0 => {
                    for byte in &buf[0..count] {
                        if let Ok(()) = uart.write(*byte) {}
                    }
                }
                _ => {}
            }
        }

        while let Ok(byte) = uart.read() {
            // TODO: lose these unwraps. Maybe just ignore errors.
            serial.write(&[byte]).unwrap();
            webusb.write(&[byte]).unwrap();
        }

        // https://github.com/espressif/esptool/wiki/ESP32-Boot-Mode-Selection#automatic-bootloader
        // DTR RTS| EN  IO0
        // 1   1  |  1   1
        // 0   0  |  1   1
        // 1   0  |  0   1
        // 0   1  |  1   0
        let dtr = serial.dtr() || webusb.dtr();
        let rts = serial.rts() || webusb.rts();

        if !(rts && dtr) {
            esp_gpio0.set_high().unwrap();
            esp_en.set_high().unwrap();
        } else {
            if dtr {
                esp_gpio0.set_high().unwrap();
            } else {
                esp_gpio0.set_low().unwrap();
            }

            if rts {
                esp_en.set_high().unwrap();
            } else {
                esp_en.set_low().unwrap();
            }
        }

    }
}
