use std::fmt::Write;

pub fn bytes_to_string_with_encoding(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(40);
    for byte in bytes {
        write!(s, "%{:02X}", byte).unwrap();
    }
    s
}
