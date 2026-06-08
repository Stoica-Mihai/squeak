"""Textual TUI for Keychron M6 config (proof-of-concept over the keycron backend).

Run:  python -m keycron.tui   (or: keycron-tui)
"""

from textual.app import App, ComposeResult
from textual.widgets import Header, Footer, DataTable

from keycron.device import Device
from keycron import block, polling, buttons


class KeycronTUI(App):
    TITLE = "Keychron M6"
    BINDINGS = [("r", "refresh", "Refresh"), ("q", "quit", "Quit")]

    def compose(self) -> ComposeResult:
        yield Header()
        yield DataTable(id="settings")
        yield DataTable(id="buttons")
        yield Footer()

    def on_mount(self) -> None:
        self.query_one("#settings", DataTable).add_columns("Setting", "Value")
        self.query_one("#buttons", DataTable).add_columns("Button", "Assignment")
        self.refresh_data()

    def action_refresh(self) -> None:
        self.refresh_data()

    def refresh_data(self) -> None:
        with Device() as d:
            b = block.read_all(d)
            hz = polling.CODE_TO_HZ.get(b["polling"]["levels"][0], "?")
            btns = buttons.get_all(d, 16)

        s = self.query_one("#settings", DataTable)
        s.clear()
        s.add_row("Battery", f"{b['battery']['percent']}%"
                  + (" (charging)" if b["battery"]["charging"] else ""))
        s.add_row("DPI presets", str(b["dpi"]["presets"]))
        s.add_row("Polling", f"{hz} Hz")
        s.add_row("LOD", str(b["sensor"]["lod"]))
        s.add_row("Scroll dir", str(b["sensor"]["scroll_dir"]))
        s.add_row("Angle snap", str(b["sensor"]["angle"]))
        s.add_row("Debounce", f"{b['debounce']['value']} ms")
        s.add_row("Sleep", f"{b['sleep_s']} s")

        t = self.query_one("#buttons", DataTable)
        t.clear()
        for x in btns:
            t.add_row(str(x["id"]), x["name"] or x["type"])


def main():
    KeycronTUI().run()


if __name__ == "__main__":
    main()
