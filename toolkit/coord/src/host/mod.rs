mod git;
mod identity;
mod process;
mod providers;

pub(crate) use git::*;
pub(crate) use identity::*;
pub(crate) use process::*;
pub(crate) use providers::*;

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").filter(|value| !value.is_empty()).map(std::path::PathBuf::from)
}
