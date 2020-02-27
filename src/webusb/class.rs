use crate::webusb::builder::DescriptorBuilder;
use core::convert::TryInto;
use core::mem;
use usb_device::class_prelude::*;
use usb_device::Result;

const CS_INTERFACE: u8 = 0x24;
const CDC_TYPE_HEADER: u8 = 0x00;
const CDC_TYPE_CALL_MANAGEMENT: u8 = 0x01;
const CDC_TYPE_ACM: u8 = 0x02;
const CDC_TYPE_UNION: u8 = 0x06;

const WEBUSB_VENDOR_CODE: u8 = 0x42;
const WEBUSB_GET_URL: u16 = 0x02;
const WEBUSB_DESCRIPTOR_URL: u8 = 0x03;
const WEBUSB_SCHEME_HTTPS: u8 = 0x01;

const MS_VENDOR_CODE: u8 = 0x43;
const MS_GET_DESCRIPTOR_SET: u16 = 0x07;

static MS_DEVICE_UUID: &str = "{f37ccce8-a70f-492a-acfb-cf2b2dab56a3}\0\0";

const REQ_SEND_ENCAPSULATED_COMMAND: u8 = 0x00;
#[allow(unused)]
const REQ_GET_ENCAPSULATED_COMMAND: u8 = 0x01;
const REQ_SET_LINE_CODING: u8 = 0x20;
const REQ_GET_LINE_CODING: u8 = 0x21;
const REQ_SET_CONTROL_LINE_STATE: u8 = 0x22;

/// Packet level implementation of a CDC-ACM serial port.
///
/// This class can be used directly and it has the least overhead due to directly reading and
/// writing USB packets with no intermediate buffers, but it will not act like a stream-like serial
/// port. The following constraints must be followed if you use this class directly:
///
/// - `read_packet` must be called with a buffer large enough to hold max_packet_size bytes, and the
///   method will return a `WouldBlock` error if there is no packet to be read.
/// - `write_packet` must not be called with a buffer larger than max_packet_size bytes, and the
///   method will return a `WouldBlock` error if the previous packet has not been sent yet.
/// - If you write a packet that is exactly max_packet_size bytes long, it won't be processed by the
///   host operating system until a subsequent shorter packet is sent. A zero-length packet (ZLP)
///   can be sent if there is no other data to send. This is because USB bulk transactions must be
///   terminated with a short packet, even if the bulk endpoint is used for stream-like data.
pub struct WebUsbClass<'a, B: UsbBus> {
    comm_if: InterfaceNumber,
    comm_ep: EndpointIn<'a, B>,
    data_if: InterfaceNumber,
    read_ep: EndpointOut<'a, B>,
    write_ep: EndpointIn<'a, B>,
    line_coding: LineCoding,
    dtr: bool,
    rts: bool,
}

impl<B: UsbBus> WebUsbClass<'_, B> {
    /// Creates a new WebUsbClass with the provided UsbBus and max_packet_size in bytes. For
    /// full-speed devices, max_packet_size has to be one of 8, 16, 32 or 64.
    pub fn new(alloc: &UsbBusAllocator<B>, max_packet_size: u16) -> WebUsbClass<'_, B> {
        WebUsbClass {
            comm_if: alloc.interface(),
            comm_ep: alloc.interrupt(8, 255),
            data_if: alloc.interface(),
            read_ep: alloc.bulk(max_packet_size),
            write_ep: alloc.bulk(max_packet_size),
            line_coding: LineCoding {
                stop_bits: StopBits::One,
                data_bits: 8,
                parity_type: ParityType::None,
                data_rate: 8_000,
            },
            dtr: false,
            rts: false,
        }
    }

    /// Gets the maximum packet size in bytes.
    pub fn max_packet_size(&self) -> u16 {
        // The size is the same for both endpoints.
        self.read_ep.max_packet_size()
    }

    /// Gets the current line coding. The line coding contains information that's mainly relevant
    /// for USB to UART serial port emulators, and can be ignored if not relevant.
    pub fn line_coding(&self) -> &LineCoding {
        &self.line_coding
    }

    /// Gets the DTR (data terminal ready) state
    pub fn dtr(&self) -> bool {
        self.dtr
    }

    /// Gets the RTS (ready to send) state
    pub fn rts(&self) -> bool {
        self.rts
    }

    /// Writes a single packet into the IN endpoint.
    pub fn write_packet(&mut self, data: &[u8]) -> Result<usize> {
        self.write_ep.write(data)
    }

    /// Reads a single packet from the OUT endpoint.
    pub fn read_packet(&mut self, data: &mut [u8]) -> Result<usize> {
        self.read_ep.read(data)
    }

    /// Gets the address of the IN endpoint.
    pub(crate) fn write_ep_address(&self) -> EndpointAddress {
        self.write_ep.address()
    }
}

impl<B: UsbBus> UsbClass<B> for WebUsbClass<'_, B> {
    fn get_configuration_descriptors(&self, writer: &mut DescriptorWriter) -> Result<()> {
        writer.interface(
            self.comm_if,
            0xFF, // Interface class: Vendor specific
            0x00, // Subclass 0
            0x00,
        )?; // Protocol 0

        writer.write(
            CS_INTERFACE,
            &[
                CDC_TYPE_HEADER, // bDescriptorSubtype
                0x10,
                0x01, // bcdCDC (1.10)
            ],
        )?;

        writer.write(
            CS_INTERFACE,
            &[
                CDC_TYPE_ACM, // bDescriptorSubtype
                0x00,         // bmCapabilities
            ],
        )?;

        writer.write(
            CS_INTERFACE,
            &[
                CDC_TYPE_UNION,      // bDescriptorSubtype
                self.comm_if.into(), // bControlInterface
                self.data_if.into(), // bSubordinateInterface
            ],
        )?;

        writer.write(
            CS_INTERFACE,
            &[
                CDC_TYPE_CALL_MANAGEMENT, // bDescriptorSubtype
                0x00,                     // bmCapabilities
                self.data_if.into(),      // bDataInterface
            ],
        )?;

        writer.endpoint(&self.comm_ep)?;

        writer.interface(
            self.data_if,
            0xFF, // Interface class: Vendor specific
            0x01, // Subclass 1
            0x00,
        )?; //Protocol 0

        writer.endpoint(&self.write_ep)?;
        writer.endpoint(&self.read_ep)?;
        Ok(())
    }

    fn get_bos_descriptors(&self, writer: &mut BosWriter) -> Result<()> {
        // WebUSB BOS descriptor
        writer.capability(
            0x05,
            &[
                0x00, // bReserved
                // UUID
                0x38,
                0xB6,
                0x08,
                0x34,
                0xA9,
                0x09,
                0xA0,
                0x47,
                0x8B,
                0xFD,
                0xA0,
                0x76,
                0x88,
                0x15,
                0xB6,
                0x65,
                0x00,               // bcdVersion (LSB)
                0x01,               // bcdVersion (MSB)
                WEBUSB_VENDOR_CODE, // bVendorCode
                0x01,               // iLandingPage
            ],
        )?;
        // Microsoft OS 2.0 platform capability descriptor
        writer.capability(
            0x05,
            &[
                0x00, // bReserved
                //UUID
                0xDF,
                0x60,
                0xDD,
                0xD8,
                0x89,
                0x45,
                0xC7,
                0x4C,
                0x9C,
                0xD2,
                0x65,
                0x9D,
                0x9E,
                0x64,
                0x8A,
                0x9F,
                0x00,           // dwWindowsVersion
                0x00,           // ..
                0x03,           // ..
                0x06,           // ..
                0xB2,           // wMSOSDescriptorSetTotalLength
                0x00,           // ..
                MS_VENDOR_CODE, // bMS_VendorCode
                0x00,           // bAltEnumCode
            ],
        )?;

        Ok(())
    }

    fn reset(&mut self) {
        self.line_coding = LineCoding::default();
        self.dtr = false;
        self.rts = false;
    }

    fn control_in(&mut self, xfer: ControlIn<B>) {
        let req = xfer.request();

        // Ignore control messages not directed at this interface, except for WebUSB
        if !(req.request_type == control::RequestType::Class
            && req.recipient == control::Recipient::Interface
            && req.index == u8::from(self.comm_if) as u16)
            && req.request != WEBUSB_VENDOR_CODE
            && req.request != MS_VENDOR_CODE
        {
            return;
        }

        match req.request {
            // REQ_GET_ENCAPSULATED_COMMAND is not really supported - it will be rejected below.
            REQ_GET_LINE_CODING if req.length == 7 => {
                xfer.accept(|data| {
                    data[0..4].copy_from_slice(&self.line_coding.data_rate.to_le_bytes());
                    data[4] = self.line_coding.stop_bits as u8;
                    data[5] = self.line_coding.parity_type as u8;
                    data[6] = self.line_coding.data_bits;

                    Ok(7)
                })
                .ok();
            }
            WEBUSB_VENDOR_CODE if req.index == WEBUSB_GET_URL => {
                // WebUSB URL descriptor (spec section 4.3)
                let url = b"tide.emfcamp.org\0"; // Does this need a null-terminator or am I off-by-one somewhere?
                xfer.accept(|data| {
                    let length = url.len() + 2;
                    data[0] = length as u8;
                    data[1] = WEBUSB_DESCRIPTOR_URL;
                    data[2] = WEBUSB_SCHEME_HTTPS;
                    data[3..3 + url.len()].copy_from_slice(url);
                    Ok(length)
                })
                .ok();
            }
            MS_VENDOR_CODE if req.index == MS_GET_DESCRIPTOR_SET => {
                let mut buf: [u8; 180] = [0; 180];
                let mut db = DescriptorBuilder::new(&mut buf);

                // Microsoft OS 2.0 descriptor set header
                db.write_u16(0x000A); // wLength
                db.write_u16(0x0000); // wDescriptorType
                db.write_u32(0x0603_0000); // dwWindowsVersion
                db.write_u16(0x00B2); // wTotalLength

                // Microsoft OS 2.0 configuration subset header
                db.write_u16(0x0008); // wLength
                db.write_u16(0x0001); // wDescriptorType
                db.write(&[0x00, 0x00]); // NOTE: hardcoded configuration 1 here
                db.write_u16(0x00A8);

                // Microsoft OS 2.0 function subset header
                db.write_u16(0x0008);
                db.write_u16(0x0002);
                db.write(&[0x02, 0x00]);
                db.write_u16(0x00A0);

                // Microsoft OS 2.0 compatible ID descriptor
                db.write_u16(0x0014);
                db.write_u16(0x0003);
                db.write(b"WINUSB\0\0");
                db.write(b"\0\0\0\0\0\0\0\0");

                // Microsoft OS 2.0 registry property descriptor
                db.write_u16(0x0084);
                db.write_u16(0x0004);
                db.write_u16(0x0007);
                db.write_u16(0x002A);
                db.write_utf16("DeviceInterfaceGUIDs\0");
                db.write_u16(0x0050);
                db.write_utf16(MS_DEVICE_UUID);

                xfer.accept_with(db.buf()).ok();
            }
            _ => {
                xfer.reject().ok();
            }
        }
    }

    fn control_out(&mut self, xfer: ControlOut<B>) {
        let req = xfer.request();

        if !(req.request_type == control::RequestType::Class
            && req.recipient == control::Recipient::Interface
            && req.index == u8::from(self.comm_if) as u16)
        {
            return;
        }

        match req.request {
            REQ_SEND_ENCAPSULATED_COMMAND => {
                // We don't actually support encapsulated commands but pretend we do for standards
                // compatibility.
                xfer.accept().ok();
            }
            REQ_SET_LINE_CODING if xfer.data().len() >= 7 => {
                self.line_coding.data_rate =
                    u32::from_le_bytes(xfer.data()[0..4].try_into().unwrap());
                self.line_coding.stop_bits = xfer.data()[4].into();
                self.line_coding.parity_type = xfer.data()[5].into();
                self.line_coding.data_bits = xfer.data()[6];

                xfer.accept().ok();
            }
            REQ_SET_CONTROL_LINE_STATE => {
                self.dtr = (req.value & 0x0001) != 0;
                self.rts = (req.value & 0x0002) != 0;

                xfer.accept().ok();
            }
            _ => {
                xfer.reject().ok();
            }
        };
    }
}

/// Number of stop bits for LineCoding
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum StopBits {
    /// 1 stop bit
    One = 0,

    /// 1.5 stop bits
    OnePointFive = 1,

    /// 2 stop bits
    Two = 2,
}

impl From<u8> for StopBits {
    fn from(value: u8) -> Self {
        if value <= 2 {
            unsafe { mem::transmute(value) }
        } else {
            StopBits::One
        }
    }
}

/// Parity for LineCoding
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ParityType {
    None = 0,
    Odd = 1,
    Event = 2,
    Mark = 3,
    Space = 4,
}

impl From<u8> for ParityType {
    fn from(value: u8) -> Self {
        if value <= 4 {
            unsafe { mem::transmute(value) }
        } else {
            ParityType::None
        }
    }
}

/// Line coding parameters
///
/// This is provided by the host for specifying the standard UART parameters such as baud rate. Can
/// be ignored if you don't plan to interface with a physical UART.
pub struct LineCoding {
    stop_bits: StopBits,
    data_bits: u8,
    parity_type: ParityType,
    data_rate: u32,
}

impl LineCoding {
    /// Gets the number of stop bits for UART communication.
    pub fn stop_bits(&self) -> StopBits {
        self.stop_bits
    }

    /// Gets the number of data bits for UART communication.
    pub fn data_bits(&self) -> u8 {
        self.data_bits
    }

    /// Gets the parity type for UART communication.
    pub fn parity_type(&self) -> ParityType {
        self.parity_type
    }

    /// Gets the data rate in bits per second for UART communication.
    pub fn data_rate(&self) -> u32 {
        self.data_rate
    }
}

impl Default for LineCoding {
    fn default() -> Self {
        LineCoding {
            stop_bits: StopBits::One,
            data_bits: 8,
            parity_type: ParityType::None,
            data_rate: 8_000,
        }
    }
}
