use super::*;

/// Scripted in-memory mouse: serves the 0x06 block + per-button slots, and
/// applies confirmable writes to its state so read-backs reflect them.
struct FakeHid {
    block: Vec<u8>,
    buttons: Vec<(u8, u32)>,
    fail: bool,     // every transfer times out
    fail_get: bool, // only reads time out (writes still ack)
    reject: bool,   // writes ack with a non-OK status
}

impl FakeHid {
    fn new() -> Self {
        FakeHid {
            block: block::sample_block(),
            buttons: vec![(0u8, 0u32); buttons::COUNT],
            fail: false,
            fail_get: false,
            reject: false,
        }
    }
}

impl Hid for FakeHid {
    fn get(&mut self, cmd: u8, payload: &[u8]) -> Result<Vec<u8>, HidError> {
        if self.fail || self.fail_get {
            return Err(HidError::Timeout(cmd));
        }
        match cmd {
            0x06 => {
                let mut r = vec![0xB4];
                r.extend_from_slice(&self.block);
                Ok(r)
            }
            0x62 => {
                let (t, d) = self.buttons[payload[0] as usize];
                Ok(vec![0xB4, 0x62, 0, 0, t, (d >> 16) as u8, (d >> 8) as u8, d as u8])
            }
            _ => Ok(vec![0xB4, cmd]),
        }
    }

    fn set(&mut self, cmd: u8, payload: &[u8]) -> Result<(bool, Vec<u8>), HidError> {
        if self.fail {
            return Err(HidError::Timeout(cmd));
        }
        if !self.reject {
            match cmd {
                0x43 => self.block[17] = payload[0],                          // debounce
                0x0A => self.block[18] = payload[1],                          // sleep [1, min]
                0x41 => self.block[2] = (payload[0] << 4) | (self.block[2] & 0x0f), // rate code
                0x40 => {
                    self.block[4] = payload[0]; // active
                    for i in 0..5 {
                        self.block[5 + 2 * i] = payload[3 + 2 * i];
                        self.block[6 + 2 * i] = payload[4 + 2 * i];
                    }
                }
                _ => {}
            }
        }
        let status = u8::from(self.reject);
        Ok((!self.reject, vec![0xB6, 0xE4, status, cmd]))
    }

    fn long_set(&mut self, cmd: u8, payload: &[u8]) -> Result<(bool, Vec<u8>), HidError> {
        if self.fail {
            return Err(HidError::Timeout(cmd));
        }
        if !self.reject && cmd == 0x52 {
            let d = ((payload[3] as u32) << 16) | ((payload[4] as u32) << 8) | payload[5] as u32;
            self.buttons[payload[0] as usize] = (payload[2], d);
        }
        let status = u8::from(self.reject);
        Ok((!self.reject, vec![0x00, 0xE4, status, cmd]))
    }

    fn long_raw(&mut self, _payload: &[u8]) -> Result<Vec<u8>, HidError> {
        if self.fail {
            return Err(HidError::Timeout(0));
        }
        Ok(vec![0x00, 0xE4, 0x00])
    }
}

#[test]
fn read_all_returns_settings() {
    let mut h = FakeHid::new();
    let u = handle(Cmd::ReadAll, &mut h);
    match u.as_slice() {
        [Update::Settings(s)] => {
            assert_eq!(s.debounce.value, 10);
            assert_eq!(s.battery.percent, 85);
        }
        _ => panic!("expected one Settings update"),
    }
}

#[test]
fn read_all_transport_error() {
    let mut h = FakeHid::new();
    h.fail = true;
    let u = handle(Cmd::ReadAll, &mut h);
    assert!(matches!(u.as_slice(), [Update::Error(_)]));
}

#[test]
fn read_buttons_returns_all_slots() {
    let mut h = FakeHid::new();
    let u = handle(Cmd::ReadButtons, &mut h);
    match u.as_slice() {
        [Update::Buttons(v)] => assert_eq!(v.len(), buttons::COUNT),
        _ => panic!("expected Buttons update"),
    }
}

#[test]
fn set_debounce_writes_then_refreshes() {
    let mut h = FakeHid::new();
    let u = handle(Cmd::SetDebounce(7), &mut h);
    assert!(matches!(u[0], Update::Written { ok: true, .. }));
    match &u[1] {
        Update::Settings(s) => assert_eq!(s.debounce.value, 7),
        _ => panic!("expected refreshed Settings"),
    }
}

#[test]
fn set_dpi_writes_then_refreshes() {
    let mut h = FakeHid::new();
    let u = handle(Cmd::SetDpi { index: 0, value: 1200 }, &mut h);
    assert!(matches!(u[0], Update::Written { ok: true, .. }));
    match &u[1] {
        Update::Settings(s) => assert_eq!(s.dpi.presets[0], 1200),
        _ => panic!("expected refreshed Settings"),
    }
}

#[test]
fn button_disable_writes_then_refreshes() {
    let mut h = FakeHid::new();
    let u = handle(Cmd::SetButtonDisable(2), &mut h);
    assert!(matches!(u[0], Update::Written { ok: true, .. }));
    match &u[1] {
        Update::Buttons(v) => assert_eq!(v[2].type_id, 9), // TYPE_DISABLE
        _ => panic!("expected refreshed Buttons"),
    }
}

#[test]
fn rejected_write_reports_failure_only() {
    let mut h = FakeHid::new();
    h.reject = true;
    let u = handle(Cmd::SetDebounce(7), &mut h);
    assert_eq!(u.len(), 1);
    assert!(matches!(u[0], Update::Written { ok: false, .. }));
}

#[test]
fn write_ok_but_readback_failure_errors() {
    // factory_reset acks (no internal read-back); the post-write refresh fails.
    let mut h = FakeHid::new();
    h.fail_get = true;
    let u = handle(Cmd::FactoryReset, &mut h);
    assert!(matches!(u[0], Update::Written { ok: true, .. }));
    assert!(matches!(u[1], Update::Error(_)));
}
