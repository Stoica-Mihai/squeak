// Pure-std hidraw proof: find the 0xFFC1 config collection, read the 0x06 block.
use std::fs;
use std::io::{Read, Write};

fn find() -> Option<String> {
    for e in fs::read_dir("/sys/class/hidraw").ok()? {
        let name = e.ok()?.file_name().into_string().ok()?;
        let dev = format!("/sys/class/hidraw/{}/device", name);
        let uevent = fs::read_to_string(format!("{}/uevent", dev)).unwrap_or_default();
        if !uevent.to_uppercase().contains("V00003434") { continue; }
        let desc = fs::read(format!("{}/report_descriptor", dev)).unwrap_or_default();
        if desc.len() >= 3 && desc[0] == 0x06 && desc[1] == 0xC1 && desc[2] == 0xFF {
            return Some(format!("/dev/{}", name));
        }
    }
    None
}

fn le16(b: &[u8], o: usize) -> u16 { (b[o] as u16) | ((b[o + 1] as u16) << 8) }

fn main() {
    let path = find().expect("config collection not found");
    println!("device: {}", path);
    let mut f = fs::OpenOptions::new().read(true).write(true).open(&path).unwrap();
    let mut out = [0u8; 64];
    out[0] = 0xB3; // long report id
    out[1] = 0x06; // GET block
    f.write_all(&out).unwrap();
    for _ in 0..50 {
        let mut r = [0u8; 64];
        let n = f.read(&mut r).unwrap();
        if n > 0 && r[0] == 0xB4 {
            let b = &r[1..];
            let battery = b[19] & 0x7f;
            let presets = [le16(b,5), le16(b,7), le16(b,9), le16(b,11), le16(b,13)];
            println!("battery = {}%", battery);
            println!("dpi presets = {:?}", presets);
            println!("debounce = {} ms, sleep = {} s", b[17], b[18]);
            return;
        }
    }
    eprintln!("no 0xB4 reply");
}
