# QR Code Generator: agent notes

- Package / manifest key: `qrcode-app` → `gam::APP_NAME_QRCODE_APP`. Server name `_QR Code Generator_`. Repo name stays `precursor-qrcode`.
- `UxType::Framebuffer` via `precursor_appkit::app::register`. No pump or timers.
- Pure logic in `core/` (`qrcode-app-core`): encoder (`encode.rs`, V1-10), settings/saved codes/migration/templates/text field (`model.rs`). `tests.rs` diffs every version × EC × mode × mask against the `qrcode` crate (dev-dependency only); any encoder change must keep that passing.
- Storage through `precursor_appkit::store::Store`: `qrcode.settings/config`, `qrcode.codes/index` (one record, so delete removes the payload). Reload on basis change.
- Every screen exits on Backspace or F4. Never log keystrokes or payloads (`SavedCode` and `QrCode` have redacting `Debug`).
- Gate: `xous-dev-toolkit/bin/integrate --xous-root <tree> --grade <this repo>`.
- `DESIGN.md` and `AGENTS.md` are 0.1 notes kept for history; README is authoritative.
