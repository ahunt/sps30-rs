use std::fmt;

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

fn unstuff_rx_data(expected_unstuffed_length: usize, data: &[u8]) -> Result<Vec<u8>, String> {
    if expected_unstuffed_length > data.len() {
        return Result::Err(String::from(
            "expected unstuffed length cannot be greater than actual stuffed length",
        ));
    }

    let mut out = Vec::with_capacity(expected_unstuffed_length);
    let mut it = data.iter();
    while let Some(byte) = it.next() {
        if *byte == 0x7D {
            let mapped = match it.next() {
                Some(0x5E) => Some(0x7E),
                Some(0x5D) => Some(0x7D),
                Some(0x31) => Some(0x11),
                Some(0x33) => Some(0x13),
                // TODO: include the stuffed byte once we have real error types.
                Some(_) => return Result::Err(String::from("invalid/unsupported stuffed byte")),
                None => {
                    return Result::Err(String::from("unexpected end of data after stuff byte"))
                }
            };
            out.push(mapped.unwrap());
        } else {
            out.push(*byte);
        }
    }
    if out.len() != expected_unstuffed_length {
        return Result::Err(String::from(
            "unstuffed length did not match expected unstuffed length",
        ));
    }
    Result::Ok(out)
}

pub struct MisoFrame {
    adr: u8,
    cmd: u8,
    state: u8,
    data: Vec<u8>,
}

impl fmt::Debug for MisoFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MisoFrame {{ adr: {}, cmd: {}, state: {}, data: {:#X?} }}",
            self.adr, self.cmd, self.state, self.data
        )
    }
}

impl PartialEq for MisoFrame {
    fn eq(&self, other: &Self) -> bool {
        self.adr == other.adr
            && self.cmd == other.cmd
            && self.state == other.state
            && self.data == other.data
    }
}

// Decode an entire miso_frame, including start/stop bytes.
pub fn decode_miso_frame(data: &[u8]) -> Result<MisoFrame, String> {
    if data.len() < 7 {
        return Result::Err(String::from("invalid miso frame length"));
    }

    if data[0] != 0x7E || data[data.len() - 1] != 0x7E {
        return Result::Err(String::from(
            "invalid miso frame: incorrect/missing start/stop bytes",
        ));
    }

    let expected_unstuffed_length = data[4];
    let rx_data = &data[5..data.len() - 2];
    let unstuffed_data = unstuff_rx_data(usize::from(expected_unstuffed_length), &rx_data)?;

    // TODO: check checksum.

    Result::Ok(MisoFrame {
        adr: data[1],
        cmd: data[2],
        state: data[3],
        data: unstuffed_data,
    })
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

    #[test]
    fn test_unstuff_rx_data() {
        struct TestCase<'a> {
            input: &'a [u8],
            expected_unstuffed_length_input: usize,
            expected_result: Result<Vec<u8>, String>,
        }
        let tests = [
            TestCase {
                input: &[],
                expected_unstuffed_length_input: 0,
                expected_result: Result::Ok(vec![]),
            },
            TestCase {
                input: &[],
                expected_unstuffed_length_input: 1,
                expected_result: Result::Err(String::from(
                    "expected unstuffed length cannot be greater than actual stuffed length",
                )),
            },
            TestCase {
                input: &[0],
                expected_unstuffed_length_input: 0,
                expected_result: Result::Err(String::from(
                    "unstuffed length did not match expected unstuffed length",
                )),
            },
            TestCase {
                input: &[0],
                expected_unstuffed_length_input: 1,
                expected_result: Result::Ok(vec![0]),
            },
            TestCase {
                input: &[0xFF, 0],
                expected_unstuffed_length_input: 2,
                expected_result: Result::Ok(vec![0xFF, 0]),
            },
            TestCase {
                input: &[0x7D, 0x5E],
                expected_unstuffed_length_input: 1,
                expected_result: Result::Ok(vec![0x7E]),
            },
            TestCase {
                input: &[0x7D, 0x5D],
                expected_unstuffed_length_input: 1,
                expected_result: Result::Ok(vec![0x7D]),
            },
            TestCase {
                input: &[0x7D, 0x31],
                expected_unstuffed_length_input: 1,
                expected_result: Result::Ok(vec![0x11]),
            },
            TestCase {
                input: &[0x7D, 0x33],
                expected_unstuffed_length_input: 1,
                expected_result: Result::Ok(vec![0x13]),
            },
            TestCase {
                input: &[0, 0x7D, 0x5E, 0],
                expected_unstuffed_length_input: 3,
                expected_result: Result::Ok(vec![0, 0x7E, 0]),
            },
            TestCase {
                input: &[0, 0x7D, 0x5E, 0],
                expected_unstuffed_length_input: 4,
                expected_result: Result::Err(String::from(
                    "unstuffed length did not match expected unstuffed length",
                )),
            },
        ];
        for case in tests {
            let out = unstuff_rx_data(case.expected_unstuffed_length_input, case.input);
            assert_eq!(case.expected_result, out)
        }
    }
    #[test]
    fn test_decode_miso_frame() {
        struct TestCase<'a> {
            input: &'a [u8],
            expected_result: Result<MisoFrame, String>,
        }
        let tests = [
            TestCase {
                input: &[],
                expected_result: Result::Err(String::from("invalid miso frame length")),
            },
            TestCase {
                // TODO: fix CHK (2nd last byte) once checksum checks are implemented.
                input: &[0x7E, 0, 0, 0, 0, 0, 0x7E],
                expected_result: Result::Ok(MisoFrame {
                    adr: 0,
                    cmd: 0,
                    state: 0,
                    data: vec![],
                }),
            },
            TestCase {
                // TODO: fix CHK (2nd last byte) once checksum checks are implemented.
                input: &[0x7E, 0, 0, 0, 1, 0xFF, 0, 0x7E],
                expected_result: Result::Ok(MisoFrame {
                    adr: 0,
                    cmd: 0,
                    state: 0,
                    data: vec![0xFF],
                }),
            },
            TestCase {
                // TODO: fix CHK (2nd last byte) once checksum checks are implemented.
                input: &[0x7E, 0, 0, 0, 4, 0xFF, 0x7D, 0x5E, 1, 0xFF, 0, 0x7E],
                expected_result: Result::Ok(MisoFrame {
                    adr: 0,
                    cmd: 0,
                    state: 0,
                    data: vec![0xFF, 0x7E, 1, 0xFF],
                }),
            },
            TestCase {
                // TODO: fix CHK (2nd last byte) once checksum checks are implemented.
                input: &[0x7E, 1, 2, 3, 1, 0xFF, 0, 0x7E],
                expected_result: Result::Ok(MisoFrame {
                    adr: 1,
                    cmd: 2,
                    state: 3,
                    data: vec![0xFF],
                }),
            },
        ];
        for case in tests {
            let out = decode_miso_frame(case.input);
            assert_eq!(case.expected_result, out)
        }
    }
}
