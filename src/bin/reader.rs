extern crate serialport;
use std::io::BufRead;
use std::io::BufReader;

// TODO: enumerate devices dynamically
const DEVICE: &str = "/dev/ttyUSB0";

fn parse_frame(buf: &Vec<u8>) -> Result<sps30rs::shdlc::MisoFrame, String> {
    sps30rs::shdlc::decode_miso_frame(buf)
}

// TODO: this needs to be moved into a new SPS30 device API.
fn write_frame(port: &mut Box<dyn serialport::SerialPort>, frame: Vec<u8>) {
    let len_written = port.write(&frame[..]).unwrap();
    if len_written != frame.len() {
        eprintln!(
            "Expected to write {} bytes, actually wrote {}.",
            frame.len(),
            len_written
        );
        std::process::exit(0);
    }
}

fn main() {
    eprintln!("SPS 30 reader binary (v{})", env!("CARGO_PKG_VERSION"));

    let mut port = serialport::new(DEVICE, /* baud_rate */ 115_200)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .timeout(core::time::Duration::new(5, 0))
        .open()
        .expect("Unable to open serial port, sorry");

    // TODO: figure out a more sensible way of doing this instead of cloning the
    // port just to be able to read and write in parallel.
    let mut reader = BufReader::new(
        port.try_clone()
            .expect("splines failed to be reticulated (failed to clone the serialport)"),
    );

    write_frame(
        &mut port,
        sps30rs::shdlc::mosi_frame(0, /* cmd: Device Information */ 0xD0, &[0x00]).unwrap(),
    );

    let mut buf = vec![];
    // TODO: replace these loops with a reusable read_frame func (or even better
    // hide it behind the new device API).
    loop {
        match reader.read_until(0x7E, &mut buf) {
            Err(e) => {
                eprintln!("failure reading data {}", e);
                continue;
            }
            Ok(1) => {
                continue;
            }
            Ok(_) => (),
        }

        match parse_frame(&buf) {
            Err(e) => eprintln!("failed to parse frame {}", e),
            Ok(frame) => {
                if frame.cmd == 0xD0 {
                    eprintln!(
                        "Received device identifier: {}",
                        std::str::from_utf8(&frame.data).unwrap()
                    );
                    break;
                } else {
                    eprintln!("Received unexpected frame {}", frame.cmd)
                }
            }
        }
    }
    buf.clear();

    write_frame(
        &mut port,
        sps30rs::shdlc::mosi_frame(
            0,
            /* cmd: Start measurement */ 0x00,
            &[
                /* subcommand, must be 0x01 */ 0x01,
                /* output as big-endian IEEE754 float values */ 0x03,
            ],
        )
        .unwrap(),
    );
    loop {
        match reader.read_until(0x7E, &mut buf) {
            Err(e) => {
                eprintln!("failure reading data {}", e);
                continue;
            }
            Ok(1) => {
                continue;
            }
            Ok(_) => (),
        }

        match parse_frame(&buf) {
            Err(e) => eprintln!("failed to parse frame {}", e),
            Ok(frame) => {
                if frame.cmd == 0x00 {
                    eprintln!(
                        "Received start measurement response (expected to be empty): {}",
                        std::str::from_utf8(&frame.data).unwrap()
                    );
                    break;
                } else {
                    eprintln!("Received unexpected frame {}", frame.cmd);
                }
            }
        }
    }
    buf.clear();

    println!("{}", sps30rs::measurement::Measurement::csv_header());
    loop {
        write_frame(
            &mut port,
            sps30rs::shdlc::mosi_frame(0, /* cmd: Start measurement */ 0x03, &[]).unwrap(),
        );

        match reader.read_until(0x7E, &mut buf) {
            Err(e) => {
                eprintln!("unexpected error {}", e);
                break;
            }
            // We expect nowt prior to the frame start.
            Ok(1) => (),
            Ok(n) => {
                eprintln!("unexpectedly read too much data, length {}", n);
                break;
            }
        }

        match reader.read_until(0x7E, &mut buf) {
            Err(e) => {
                eprintln!("unexpected error {}", e);
                break;
            }
            Ok(_) => (),
        }

        match parse_frame(&buf) {
            Err(e) => eprintln!("failed to parse frame {}", e),
            Ok(frame) => {
                if frame.cmd == 0x03 {
                    match sps30rs::measurement::decode_measurement_frame(&frame) {
                        Ok(measurement) => println!("{}", measurement.csv_row()),
                        Err(e) => eprintln!("failed to decode measurement: {}", e),
                    }
                } else {
                    eprintln!("Received unexpected frame {}", frame.cmd);
                }
            }
        }
        std::thread::sleep(std::time::Duration::new(5, 0));
        buf.clear();
    }
}
