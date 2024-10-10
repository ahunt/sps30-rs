# SPS 30 Rust Library

This is nowhere near usable (yet?).

## Known issues

* The SPS30 sometimes switches into a mode where it returns no data, for a
  long time (i.e. the read measurement response is always empty). It may, or may
  not, recover. This appears to be a hardware issue (or perhaps an issue with
  the cabling I'm using, power supply, etc.). Examples:

  * https://github.com/Sensirion/arduino-sps/issues/14
  * https://github.com/Sensirion/arduino-sps/issues/30
