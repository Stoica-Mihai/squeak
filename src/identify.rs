//! `squeak identify` — map physical buttons to slot ids. Sets each present
//! button to a macro that types its own id, waits while you press each physical
//! button into a text editor, then restores every button to its prior value.

use std::io::{Write, stdin, stdout};

use anyhow::{Context, Result, bail};

use crate::hid::{Device, find_config};
use crate::proto::buttons::{self, is_present};
use crate::proto::macros;

pub fn run() -> Result<()> {
    let info = find_config().context("Keychron config device not found (plug in the dongle)")?;
    let mut dev = Device::open(&info.node).context("open hidraw")?;

    let original = buttons::get_all(&mut dev, buttons::COUNT).context("read buttons")?;
    let present: Vec<_> = original.iter().filter(|b| is_present(b)).collect();
    if present.is_empty() {
        bail!("no configurable buttons found");
    }

    println!("squeak identify — temporarily remapping {} buttons.\n", present.len());
    println!("⚠ left/right click will be remapped during this — use the keyboard.\n");
    for b in &present {
        let events = macros::text_events(&b.id.to_string())?;
        macros::set_macro(&mut dev, b.id, &events)
            .with_context(|| format!("set identify macro on id {}", b.id))?;
        println!("  id {:>2}  → types \"{}\"", b.id, b.id);
    }

    println!("\nOpen a text editor, press EACH physical button, and note the number it types.");
    print!("Then press Enter here to restore all buttons… ");
    stdout().flush().ok();
    let mut line = String::new();
    stdin().read_line(&mut line).ok();

    println!("\nrestoring…");
    for b in &present {
        let r = if b.type_id == 0 {
            buttons::restore_default(&mut dev, b.id).map(|_| ())
        } else {
            buttons::set_button(&mut dev, b.id, b.type_id, b.data).map(|_| ())
        };
        match r {
            Ok(()) => println!("  id {:>2}  restored", b.id),
            Err(e) => eprintln!("  id {:>2}  RESTORE FAILED: {e}", b.id),
        }
    }
    println!("\ndone. Tell me the physical→id mapping and I'll label them.");
    Ok(())
}
