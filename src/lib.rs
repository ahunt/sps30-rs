// TODO: temporarily allow dead_code until basic implementation is done.
#![allow(dead_code)]

// TODO: temporarily make public until the real API has been determined.
pub mod measurement;
pub mod shdlc;

// TODO: remove default boilerplate.
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
