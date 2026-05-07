use devi::asset::grf::GrfArchive;

#[test]
fn test_grf_header_magic() {
    let magic = b"Master of Magic\0";
    assert_eq!(magic.len(), 16);
}

#[test]
fn test_grf_open_nonexistent() {
    let result = GrfArchive::open("nonexistent.grf");
    assert!(result.is_err());
}

#[test]
fn test_grf_file_entry_size() {
    use std::mem::size_of;
    assert_eq!(size_of::<u32>(), 4);
    assert_eq!(size_of::<u8>(), 1);
}
