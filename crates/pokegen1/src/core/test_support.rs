//! Shared #[cfg(test)] helpers for building synthetic SRAM buffers.
pub(crate) const SRAM_LEN: usize = 0x8000; // a real Gen-1 .sav is 32 KiB
pub(crate) fn blank_sram() -> Vec<u8> {
    vec![0u8; SRAM_LEN]
}
pub(crate) fn seed(buf: &mut [u8], offset: usize, bytes: &[u8]) {
    buf[offset..offset + bytes.len()].copy_from_slice(bytes);
}
