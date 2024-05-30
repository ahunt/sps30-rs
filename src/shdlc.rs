/// stuff_data stuffs data following SHDLC conventions.
///
/// Or, to be more precise, data is stuffed following the convention documented
/// in [the SPS30 datasheet][sps30_datasheet] as that's what I had available.
///
/// Note that SHDLC "detailed protocol document is not publicly available (yet)"
/// [according to Sensirion][shdlc_python_driver_docs], although they do publish
/// a [Python Driver][python-shdlc-driver].
///
/// [sps30_datasheet]: <https://sensirion.github.io/python-shdlc-driver/shdlc.html>
/// [shdlc_python_driver_docs]: <https://sensirion.github.io/python-shdlc-driver/shdlc.html>
/// [python-shdlc-driver]: <https://github.com/Sensirion/python-shdlc-driver?tab=readme-ov-file>
fn stuff_data(data: &[u8], out: &mut Vec<u8>) {
    for byte in data {
        let mapped = match byte {
            0x7E => Some(0x5E),
            0x7D => Some(0x5D),
            0x11 => Some(0x31),
            0x13 => Some(0x33),
            _ => None,
        };
        if let Some(mapped_byte) = mapped {
            out.push(0x7D);
            out.push(mapped_byte);
        } else {
            out.push(*byte);
        }
    }
}

fn checksum(data: &[u8]) -> u8 {
    let mut sum: u8 = 0;
    for i in data {
        sum = sum.wrapping_add(*i)
    }
    !sum
}

pub fn mosi_frame(adr: u8, cmd: u8, data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() > 255 {
        return Result::Err(String::from("input too large"));
    }
    // output length won't be known until we've performed stuffing, but at least
    // we know the minimum output length.
    let mut out = Vec::with_capacity(data.len() + 6);

    out.push(0x7E); // Start
    out.push(adr);
    out.push(cmd);
    out.push(data.len().try_into().unwrap()); // specified as _unstuffed_ len.
    stuff_data(data, &mut out);
    // Checksum is on adr + cmd + len + data (excluding start/stop and the
    // checksum byte that we obviously don't know yet.)
    out.push(checksum(&out[1..]));
    out.push(0x7E); // Stop

    Result::Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stuffing() {
        struct TestCase<'a> {
            input: &'a [u8],
            expected_output: &'a [u8],
        }
        let tests = [
            TestCase {
                input: &[],
                expected_output: &[],
            },
            TestCase {
                input: &[0x00],
                expected_output: &[0x00],
            },
            TestCase {
                input: &[0xFF],
                expected_output: &[0xFF],
            },
            TestCase {
                input: &[0x00, 0xFF, 0x10],
                expected_output: &[0x00, 0xFF, 0x10],
            },
            TestCase {
                input: &[0x7E],
                expected_output: &[0x7D, 0x5E],
            },
            TestCase {
                input: &[0x7D],
                expected_output: &[0x7D, 0x5D],
            },
            TestCase {
                input: &[0x11],
                expected_output: &[0x7D, 0x31],
            },
            TestCase {
                input: &[0x13],
                expected_output: &[0x7D, 0x33],
            },
            TestCase {
                input: &[0, 0, 0, 0x7E, 1, 1, 1],
                expected_output: &[0, 0, 0, 0x7D, 0x5E, 1, 1, 1],
            },
        ];
        for case in tests {
            let mut out = Vec::new();
            stuff_data(case.input, &mut out);
            assert_eq!(case.expected_output, out);
        }
    }

    #[test]
    fn test_checksum() {
        assert_eq!(checksum(&[0x00, 0x00, 0x00]), 0xFF);
        assert_eq!(checksum(&[0x0F, 0xF0, 0x00]), 0x00);
        // Example from datasheet:
        assert_eq!(checksum(&[0x00, 0x00, 0x02, 0x01, 0x03]), 0xF9);
    }

    #[test]
    fn test_mosi_frame() {
        struct TestCase<'a> {
            input: &'a [u8],
            adr: u8,
            cmd: u8,
            expected_result: Result<Vec<u8>, String>,
        }
        let tests = [
            TestCase {
                input: &[],
                adr: 0,
                cmd: 0,
                expected_result: Result::Ok(vec![0x7E, 0, 0, 0, 0xFF, 0x7E]),
            },
            TestCase {
                input: &[0x01, 0x03],
                adr: 0,
                cmd: 0,
                expected_result: Result::Ok(vec![0x7E, 0, 0, 0x02, 0x01, 0x03, 0xF9, 0x7E]),
            },
            TestCase {
                input: &[0; 256],
                adr: 0,
                cmd: 0,
                expected_result: Result::Err(String::from("input too large")),
            },
        ];
        for case in tests {
            let out = mosi_frame(case.adr, case.cmd, case.input);
            assert_eq!(case.expected_result, out);
        }
    }
}
