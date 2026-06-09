use super::*;
use crate::proto::block;
use crate::proto::buttons::ButtonInfo;
use std::sync::mpsc::{Receiver, channel};

fn app() -> App {
    let (tx, _rx) = channel();
    App::new(tx)
}

fn sample_buttons() -> Vec<ButtonInfo> {
    (0..16u8)
        .map(|id| {
            let (type_id, data) = match id {
                5 | 10 => (3, 0xe900ff),
                6 | 11 => (3, 0xea00ff),
                12 | 15 => (0, 0),
                _ => (0, 0xffffff),
            };
            ButtonInfo { id, type_id, data, label: "x".into() }
        })
        .collect()
}

/// App with a connected device + populated snapshot; returns the Cmd receiver.
fn connected() -> (App, Receiver<Cmd>) {
    let (tx, rx) = channel();
    let mut a = App::new(tx);
    a.apply(Update::Connected {
        name: "Keychron Ultra-Link 8K".into(),
        variant: Variant::EightKNordic,
        firmware: "0.1.6".into(),
        transport: "2.4 GHz",
    });
    a.apply(Update::Settings(Box::new(block::sample_settings())));
    a.apply(Update::Buttons(sample_buttons()));
    while rx.try_recv().is_ok() {} // drain any lazy-load requests
    (a, rx)
}

#[test]
fn vertical_navigates_sections_when_sidebar_focused() {
    let mut a = app();
    assert_eq!(a.screen_idx, 0);
    a.update(Action::Vertical(1));
    assert_eq!(a.screen_idx, 1);
    a.update(Action::Vertical(-1));
    assert_eq!(a.screen_idx, 0);
    a.update(Action::Vertical(-1)); // wraps to last
    assert_eq!(a.screen_idx, Screen::ALL.len() - 1);
}

#[test]
fn enter_focuses_content_only_on_interactive_screen() {
    let mut a = app();
    a.update(Action::Enter); // Overview not interactive
    assert_eq!(a.focus, Focus::Sidebar);
    a.update(Action::Vertical(1));
    assert_eq!(a.screen(), Screen::Dpi);
    a.update(Action::Enter);
    assert_eq!(a.focus, Focus::Content);
}

#[test]
fn vertical_moves_cursor_when_content_focused() {
    let mut a = app();
    a.update(Action::Vertical(1)); // -> DPI
    a.update(Action::Enter); // focus content
    let section = a.screen_idx;
    a.update(Action::Vertical(1));
    assert_eq!(a.screen_idx, section, "section must not change in content focus");
    assert_eq!(a.dpi_cursor, 1);
    a.update(Action::Back);
    assert_eq!(a.focus, Focus::Sidebar);
}

#[test]
fn theme_picker_previews_and_reverts() {
    let (mut a, _rx) = connected();
    assert_eq!(a.theme_idx, 0);
    a.update(Action::CycleTheme); // open picker (saves 0)
    assert!(matches!(a.modal, Some(Modal::ThemePicker)));
    a.update(Action::Vertical(1)); // live preview -> Gruvbox
    assert_eq!(a.theme_idx, 1);
    a.update(Action::Back); // revert + close
    assert_eq!(a.theme_idx, 0);
    assert!(a.modal.is_none());
    // confirm path keeps the previewed theme
    a.update(Action::CycleTheme);
    a.update(Action::Vertical(2));
    a.update(Action::Enter);
    assert_eq!(a.theme_idx, 2);
    assert!(a.modal.is_none());
}

#[test]
fn dpi_enter_opens_input_and_applies() {
    let (mut a, rx) = connected();
    a.update(Action::Vertical(1)); // DPI
    a.update(Action::Enter); // focus content
    a.update(Action::Enter); // open numeric input
    assert!(matches!(a.modal, Some(Modal::DpiInput)));
    for c in "1200".chars() {
        a.input_char(c);
    }
    a.input_commit();
    match rx.try_recv().unwrap() {
        Cmd::SetDpi { index, value } => {
            assert_eq!(index, 0);
            assert_eq!(value, 1200);
        }
        other => panic!("expected SetDpi, got {other:?}"),
    }
    assert!(a.modal.is_none());
}

#[test]
fn sensor_toggle_marks_diff_and_confirm_applies() {
    let (mut a, rx) = connected();
    for _ in 0..3 {
        a.update(Action::Vertical(1)); // -> Sensor
    }
    assert_eq!(a.screen(), Screen::Sensor);
    a.update(Action::Enter); // content
    a.update(Action::Vertical(1));
    a.update(Action::Vertical(1)); // cursor -> Motion sync
    assert!(a.sensor_diff().is_empty());
    a.update(Action::Toggle); // flip motion
    assert!(!a.sensor_diff().is_empty());
    a.update(Action::Enter); // diff confirm modal
    assert!(matches!(a.modal, Some(Modal::ConfirmSensor)));
    a.update(Action::Enter); // confirm -> apply
    assert!(matches!(rx.try_recv().unwrap(), Cmd::SetSensor(_)));
    assert!(a.modal.is_none());
}

#[test]
fn button_picker_disable_sends_cmd() {
    let (mut a, rx) = connected();
    for _ in 0..4 {
        a.update(Action::Vertical(1)); // -> Buttons
    }
    assert_eq!(a.screen(), Screen::Buttons);
    a.update(Action::Enter); // content
    a.update(Action::Enter); // open picker on button 0
    assert!(matches!(a.modal, Some(Modal::ButtonPicker(_))));
    a.update(Action::Vertical(1));
    a.update(Action::Vertical(1)); // type list: Mouse, Media, -> Disable
    while rx.try_recv().is_ok() {}
    a.update(Action::Enter); // commit Disable
    match rx.try_recv().unwrap() {
        Cmd::SetButtonDisable(id) => assert_eq!(id, 0),
        other => panic!("expected SetButtonDisable, got {other:?}"),
    }
    assert!(a.modal.is_none());
}

#[test]
fn firmware_check_compares_versions() {
    let (mut a, _rx) = connected();
    a.apply(Update::Firmware { latest: Some("0.1.6".into()) });
    assert!(matches!(a.fw_check, FwCheck::UpToDate));
    a.apply(Update::Firmware { latest: Some("0.1.7".into()) });
    assert!(matches!(a.fw_check, FwCheck::Available(ref v) if v == "0.1.7"));
    a.apply(Update::Firmware { latest: None });
    assert!(matches!(a.fw_check, FwCheck::Failed));
}

#[test]
fn refresh_status_clears_when_data_lands() {
    let (mut a, _rx) = connected();
    a.update(Action::Refresh);
    assert_eq!(a.status.text, "refreshing…");
    a.apply(Update::Settings(Box::new(block::sample_settings())));
    assert!(a.status.text.starts_with("connected:"));
}

#[test]
fn reset_modal_confirm_sends_factory_reset() {
    let (mut a, rx) = connected();
    a.update(Action::ResetPrompt);
    assert!(matches!(a.modal, Some(Modal::ConfirmReset)));
    a.update(Action::Confirm);
    assert!(matches!(rx.try_recv().unwrap(), Cmd::FactoryReset));
    assert!(a.modal.is_none());
}
