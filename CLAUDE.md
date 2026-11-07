# welffi macros

welffi's Roblox macros. Rust and egui, like welffi hub. It finds Roblox, sizes the window, then plays recorded inputs when it sees a picture on screen. The hub downloads and opens it.

It was ported from the AutoHotkey macro in `Downloads\AutoHotKey\welffis macros`. The Accounts page there was left out, since welffi accounts does that job.

## Commands

`cargo` may not be on PATH. If so, use `%USERPROFILE%\.cargo\bin\cargo.exe`.

- Build: `cargo build`
- Checks: `cargo test`
- Release: `build.bat`, which makes `dist\welffi macros.exe`. Close the app first, since a running exe can't be replaced.
- Publish: raise `version` in `Cargo.toml`, run `build.bat`, then make a GitHub release on welffi/welffi-macros with a tag like `v0.1.0`. Upload `dist\welffi macros.exe` named `welffi-macros.exe`. The hub looks for that asset name.
- The exe icon is `assets\app.ico`, added by `build.rs` with `rc.exe` from the Windows SDK. The window icon is `assets\icon64.rgba`.

## Where code goes

- `roblox.rs`: finding Roblox windows, their size, focus and minimize. `fit.rs`: leaving fullscreen and sizing the client area.
- `capture.rs` grabs the screen. `vision.rs` finds a template in it.
- `recording.rs`: the WELFFI2 file format. `recorder.rs` records, `playback.rs` plays. `input.rs` sends keys and mouse. `keys.rs` maps key names to virtual keys.
- `ctd.rs` and `recorder_mode.rs`: one file per mode. `ctd.rs` runs both CTD variants (Easy and Easy Clear). Each runs its job on a `worker.rs` thread.
- `hotkeys.rs`: the global keys. `store.rs`: the data folder, built-in files and settings. `log.rs`: the log on screen.
- `app.rs`: app state and actions. `view.rs` and `panels.rs` draw it. `widgets.rs` has the shared controls and `theme.rs` every color.

Drawing code shows things and calls actions. Logic with real branches goes in a plain function with a check, so it can be tested without a window.

## Testing

welffi does the feel testing. Don't spend time on it. For UI changes, build, run the checks, take one screenshot if the look changed, then run `build.bat` and hand it over.

When running the app for a test, set `WMACROS_DATA` to a scratch folder. Only one copy can run at a time. Don't start a mode or record while Roblox is open unless the task needs it, since playback moves the real mouse.

## Data

- `%LOCALAPPDATA%\welffi macros\`: `settings.json`, `recordings\` and `templates\`.
- The CTD templates and recordings are built into the exe from `builtin\`. They're written to the data folder on start if missing, so edits there are kept.
- Recording files must stay readable by the AHK macro: header `WELFFI2 poll width height`, then `x,y;keys;buttons;wheel;t` per line. Key names follow AutoHotkey.

## Things to know

- Ctrl+R and Ctrl+P are only registered while the Recorder is open, so other apps keep them the rest of the time.
- Ctrl+F1 and Ctrl+Esc are handled on the hotkey thread, since a hidden window may not redraw.
- Fit puts the client area at 0,0. Display scaling above 100% can stop Roblox getting small enough.

## Style for welffi

- Plain voice in docs and notes: simple sentences, no em dashes. No comments in code.
- UI ideas shouldn't come from Tandem.
- Commit or push only when asked. Commits use the noreply email, so no personal address goes public.
