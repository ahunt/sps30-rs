extern crate serialport;

// TODO: enumerate devices dynamically
const DEVICE: &str = "/dev/ttyUSB0";

fn contains_frame(buf: &[u8], loc: usize) -> bool {
    let mut count = 0;
    for i in 0..loc {
        if buf[i] == 0x7E {
            count += 1;
        }
        if count == 2 {
            return true;
        }
    }
    false
}

fn read_frame(buf: &mut [u8; 512], loc: &mut usize) -> Result<sps30rs::shdlc::MisoFrame, String> {
    let mut start: Option<usize> = None;
    let mut end: Option<usize> = None;
    for i in 0..*loc {
        if buf[i] == 0x7E {
            if start == None {
                start = Some(i);
            } else {
                end = Some(i);
                break;
            }
        }
    }
    if end == None {
        return Result::Err(String::from("no frame in data"));
    }

    let frame = sps30rs::shdlc::decode_miso_frame(&buf[start.unwrap()..end.unwrap() + 1]);
    let remaining_data = *loc - (end.unwrap() + 1);
    if remaining_data > 0 {
        buf.copy_within(end.unwrap() + 1..=*loc, 0);
    }
    *loc = remaining_data;
    frame
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

    write_frame(
        &mut port,
        sps30rs::shdlc::mosi_frame(0, /* cmd: Device Information */ 0xD0, &[0x00]).unwrap(),
    );

    // TODO: this buffer should also be folded into the new SPS30 device API.
    let mut buf: [u8; 512] = [0; 512];
    let mut loc: usize = 0;
    while !contains_frame(&buf, loc) {
        loc += port.read(&mut buf[loc..]).unwrap();
    }
    let frame = read_frame(&mut buf, &mut loc).unwrap();
    eprintln!(
        "Device identifier: {}",
        std::str::from_utf8(&frame.data).unwrap()
    );

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
    while !contains_frame(&buf, loc) {
        loc += port.read(&mut buf[loc..]).unwrap();
    }
    let frame = read_frame(&mut buf, &mut loc).unwrap();
    eprintln!(
        "\nGot start Measurement response - should be empty (unless start measurement was already performed during a previous invocation): {}\n",
        std::str::from_utf8(&frame.data).unwrap()
    );

    println!("{}", sps30rs::measurement::Measurement::csv_header());
    loop {
        write_frame(
            &mut port,
            sps30rs::shdlc::mosi_frame(0, /* cmd: Start measurement */ 0x03, &[]).unwrap(),
        );

        while !contains_frame(&buf, loc) {
            loc += port.read(&mut buf[loc..]).unwrap();
        }
        // TODO: handle no data available
        let frame = read_frame(&mut buf, &mut loc).unwrap();
        let measurement = sps30rs::measurement::decode_measurement_frame(&frame).unwrap();
        println!("{}", measurement.csv_row());
        std::thread::sleep(std::time::Duration::new(5, 0));
    }
}
