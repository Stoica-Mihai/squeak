use crate::app::App;
use crate::event::Action;
use crate::proto::Variant;
use crate::proto::block;
use crate::proto::buttons::ButtonInfo;
use crate::worker::Update;
use ratatui::{Terminal, backend::TestBackend};

fn buttons() -> Vec<ButtonInfo> {
    (0..7u8)
        .map(|id| ButtonInfo { id, type_id: 0, data: 0xffffff, label: "default".into() })
        .collect()
}

fn connected() -> App {
    let (tx, _rx) = std::sync::mpsc::channel();
    let mut a = App::new(tx);
    a.apply(Update::Connected {
        name: "Keychron Ultra-Link 8K".into(),
        variant: Variant::EightKNordic,
        firmware: "0.1.6".into(),
        transport: "2.4 GHz",
    });
    a.apply(Update::Settings(Box::new(block::sample_settings())));
    a.apply(Update::Buttons(buttons()));
    a
}

/// Render the whole UI to a string (cell symbols concatenated, no newlines).
fn screenshot(a: &App) -> String {
    let mut term = Terminal::new(TestBackend::new(120, 40)).unwrap();
    term.draw(|f| super::render(f, a)).unwrap();
    term.backend().buffer().content().iter().map(|c| c.symbol()).collect()
}

#[test]
fn sidebar_and_footer_render() {
    let s = screenshot(&connected());
    assert!(s.contains("squeak"));
    assert!(s.contains("Overview"));
    assert!(s.contains("Buttons"));
    assert!(s.contains("quit"));
}

#[test]
fn overview_shows_device_and_battery() {
    let s = screenshot(&connected());
    assert!(s.contains("Keychron Ultra-Link 8K"));
    assert!(s.contains("Battery"));
    assert!(s.contains("85%"));
    assert!(s.contains("Polling"));
}

#[test]
fn dpi_screen_lists_presets() {
    let mut a = connected();
    a.update(Action::Vertical(1)); // -> DPI
    let s = screenshot(&a);
    assert!(s.contains("400"));
    assert!(s.contains("range 50"));
}

#[test]
fn sensor_screen_shows_rows() {
    let mut a = connected();
    for _ in 0..3 {
        a.update(Action::Vertical(1)); // -> Sensor
    }
    let s = screenshot(&a);
    assert!(s.contains("Lift-off distance"));
    assert!(s.contains("Motion sync"));
}

#[test]
fn buttons_screen_shows_friendly_names() {
    let mut a = connected();
    for _ in 0..4 {
        a.update(Action::Vertical(1)); // -> Buttons
    }
    let s = screenshot(&a);
    assert!(s.contains("Left"));
    assert!(s.contains("assignment"));
}

#[test]
fn help_modal_renders() {
    let mut a = connected();
    a.update(Action::Help);
    let s = screenshot(&a);
    assert!(s.contains("Help"));
    assert!(s.contains("navigate sections"));
}

#[test]
fn disconnected_shows_connecting() {
    let (tx, _rx) = std::sync::mpsc::channel();
    let a = App::new(tx);
    let s = screenshot(&a);
    assert!(s.contains("squeak")); // sidebar still drawn
}
