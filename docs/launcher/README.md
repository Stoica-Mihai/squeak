# Keychron Launcher bundles (reverse-engineering source)

The Keychron Launcher (<https://launcher.keychron.com/>) is a WebHID app — all
device logic runs client-side in these JS bundles. They are the source the
protocol maps in this `docs/` directory were decoded from. Archived here so the
RE is reproducible if Keychron rotates the hashes (they do — these filenames are
content-hashed and 404 after a redeploy).

Fetched 2026-06-09 from `https://launcher.keychron.com/<name>`:

- **`main.4dd171860f4006e1.js`** — the app. Contains the mouse command map
  (short channel `0xB5`/`0xB3`), the device→product mapping
  (`vendorProductId = vid*65536 + pid`), and the firmware-update API
  (`GET launcher.keychron.cn/api/merchandise/product/vpId/{vpId}` →
  `data.firmware.lasted.version`). Minified; grep it.
- **`scripts.e34e0ee36050e207.js`** — vendored libs, incl. the USB DFU
  firmware-flashing code (`dfuseCommand`, "Manifesting new firmware").

The only network hosts in the app: `launcher.keychron.cn` (API),
`sysmgr.keychron.cn` (firmware `.bin` files), `googletagmanager.com` (analytics).

See [`../8k-nordic.md`](../8k-nordic.md) for the decoded command map and
[`../../FINDINGS.md`](../../FINDINGS.md) for the RE log.
