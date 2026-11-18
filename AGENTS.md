# AGENTS.md

Community apps for the CMF Watch Pro 2: one Rust crate per app in `apps/<name>/`, built with the
[cmf-watch-sdk](https://github.com/cmf-tech-community/cmf-watch-sdk) at the commit the `Makefile` pins
(`WP2_APPS_REV`). CI builds merged apps and publishes them to the FMC store. README.md is the human guide; this file
is what an agent needs on top.

## Read first

`make sdk` checks the pinned SDK out to `.sdk/wp2-apps`. Its docs match the SDK you build with:

- `.sdk/wp2-apps/docs/app-guide.md`: the manual (app model, `Api`, widget and page, stock look, storage, phone
  channel, budgets, the shipping checklist in section 7).
- `.sdk/wp2-apps/docs/abi-1.4.md`: the channel key K, heart rate, pager, cards, stub version.
- `apps/vibration-lab` (template, canvas), `apps/ui-gallery` (stock LVGL widgets), `apps/sky-hop` (a game): worked
  examples. More in `.sdk/wp2-apps/sdk/examples` (`hello` and `herdr-agents` have a widget and a page).

**The SDK moves faster than the pin.** The owner publishes new loaders and stubs often, and the pinned docs only know
what existed at `WP2_APPS_REV`. Before telling anyone the watch can't do something, look at what is newer:

```bash
git -C .sdk/wp2-apps fetch --quiet origin && git -C .sdk/wp2-apps log --oneline HEAD..origin/main
```

If a newer commit has it, say so and move the pin first (skill `sdk-update`).

LVGL apps use the system font and the stock look (`cmf_wp2_app::stock`), never LVGL's defaults or an own palette.

## What an app can be

One package can offer several of these; ask which the owner wants before building.

| Surface | How | Loader |
|---|---|---|
| Menu app | the default: a row in Apps, `main` runs until `BUTTON` | any |
| Widget card | `app_features!(WIDGET)`; `api.widget(title, value)` stores text (23 B / 39 B, `\n` = 2nd line) the stub draws on a 344×166 card left of the face. A NAND write: on change, never per frame. A tap opens the app | 2026-11-15 |
| Page | `app_features!(PAGE)`: the app is a whole screen next to the face. `api.on_page()` is `true` there; no `BUTTON` (the app never exits itself), swipe away = `PAUSE`, back = `RESUME` | 2026-11-15 |
| Phone or Mac link | `send`/`recv`, `MESSAGE`: section "Apps that talk to a phone" below | 2026-11-13 for K |
| Phone music remote | LVGL only: `api.media_bind(title, artist)` fills two labels with the phone's track and sends `MEDIA` (playing, available); `api.media_key(MEDIA_PLAY…MEDIA_NEXT)`; or the whole stock tile with `stock::music`. No album art, no volume level. `abi-1.4.md`, "Music" | 2026-11-16 |

The user adds a widget or a page on the watch: long press on the app's row in Apps → "Add widget" / "Add page".
Details: `app-guide.md`, "Widget" and "Page". A "full-screen widget" means a page.

## Loop

```bash
make new NAME=my-app     # copy of vibration-lab
make test APP=my-app     # host unit tests
make sim APP=my-app      # simulator window: canvas apps only
make build APP=my-app    # target/my-app.wp2app
make check               # store.toml / store.md
```

- **The simulator has no LVGL** (`lv_submit` returns `-95`), no widget card and no page (`on_page()` is `false`). An
  LVGL app is tested by host tests of its batch builder (each batch validates against the loader's checker) and in
  the emulator. Pick canvas (`cmf-wp2-ui`) when the simulator matters more than stock widgets: `app-guide.md`,
  "LVGL or `cmf-wp2-ui`?".
- `make build` needs `resvg`, `llvm-objcopy` (rustup `llvm-tools`) and a C linker; `make doctor` lists what is missing.
- Emulator and watch targets (`emu`, `run`, `list`) need `CMF_WATCH_UTILS`, `STOCK_NOR` and `WP2_NETSIMD`: README.md.

## Skills

Step-by-step recipes in `.claude/skills/` (plain Markdown, any agent can follow them):

- `watch-app`: a new app or a change to one, from choosing its surfaces to the PR.
- `sdk-update`: move `WP2_APPS_REV` to a newer cmf-watch-sdk and teach this file what it brought.

## Hard rules

1. No writes to a physical watch (`make run ADDRESS=…`, `remove`, firmware) without the owner's explicit yes.
   The simulator and the emulator are fine.
2. No prebuilt `.wp2app`, `.bin`, ELF or firmware in commits; no personal data (MAC, serial, pairing secret, keys,
   GPS tracks, notification text). Placeholder MAC: `AA:BB:CC:DD:EE:FF`.
3. Nothing from cmf-watch-utils (AGPL-3.0) is copied here; this repository is MIT.
4. One app per PR. Every change to an app raises `version` in its `store.toml`, or CI fails. An SDK bump is a PR
   of its own.
5. The app stays inside the `Api` table: nothing keeps the motor, the screen or a sensor on after it exits.

## Apps that talk to a phone

Watch side: `send`/`recv` and the `MESSAGE` event (`app-guide.md`, section 4, with its protocol design rules).
Treat every message as untrusted: anyone in BLE range can read the pairing secret and send messages. For the same
reason nothing personal goes over the channel: no names, event or message titles, locations. Send counts, flags,
times of day.

Phone side, next to the Nothing X app on the same phone: the channel key K (`abi-1.4.md`, "Writing a host"). No
bind, no "Paired" popup, Nothing X keeps working. What exists today:

- The watch needs the stub with K (`fw-wp2.bin` from 2026-11-13 or later, by OTA) and a loader from 2026-11-13 or
  later. Stock firmware has no channel at all.
- Host implementations: Python (`wp2apps.channel.Channel(channel_key=True)`, Mac) and an iOS one in a private app.
  There is no Android host yet; `connectGatt` next to Nothing X is untested.
- A Mac and a phone can't both be connected: the watch takes one central.

An app that needs a companion says so under `## Requirements` in `store.md`, naming the companion and where to get
it.
