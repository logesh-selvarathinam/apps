# CMF Watch Pro 2 apps

Community apps for the CMF Watch Pro 2, as listed in the [FMC store](https://fmc.freethinkel.dev/apps). Each app is a
Rust crate under `apps/`, built with [cmf-watch-sdk](https://github.com/cmf-tech-community/cmf-watch-sdk). Open a pull
request with yours; once it's merged, CI builds it and publishes it to the store.

An app runs on a watch with the patched firmware and the App Loader, both on the store's
[firmware shelf](https://fmc.freethinkel.dev/firmware).

| App | What it is |
|---|---|
| [`vibration-lab`](apps/vibration-lab) | A test bench for the vibration motor. The template for new apps |
| [`ui-gallery`](apps/ui-gallery) | The stock firmware's widgets as an app can use them |
| [`tide`](apps/tide) | Music remote for your phone with a drifting ocean glow |

## Make your own app

You need git, [uv](https://docs.astral.sh/uv/), [Rust](https://rustup.rs) with
`rustup target add thumbv7em-none-eabi` and `rustup component add llvm-tools`, and
[resvg](https://github.com/linebender/resvg) for the icon. `make doctor` tells you what's missing.

```bash
make new NAME=my-app        # apps/my-app, a copy of vibration-lab
make test APP=my-app        # its host unit tests
make sim APP=my-app         # run it in a window on your computer (canvas apps, not LVGL yet)
make build APP=my-app       # target/my-app.wp2app
```

The first `make` fetches cmf-watch-sdk at the commit the Makefile pins into `.sdk/wp2-apps`: the SDK crates, the
simulator and the packaging tools all come from there, so everyone builds the same bytes.

Then write the app. [docs/app-guide.md](https://github.com/cmf-tech-community/cmf-watch-sdk/blob/main/docs/app-guide.md)
in cmf-watch-sdk is the manual: the app model, the `Api` table, the stock look (`cmf_wp2_app::stock`), storage, the phone
channel, budgets. The apps here are worked examples of it.

- The crate name is the package name and the store's key: 1–23 lowercase letters, digits, `-` and `_`. It can't
  change later.
- `icon.svg` (96×96 viewBox) is the watch menu's icon and, rendered at 288 px, the store's. Images it references are
  resolved next to it. Style: [docs/app-icons.md](https://github.com/cmf-tech-community/cmf-watch-sdk/blob/main/docs/app-icons.md).
- Use the system font and the stock look, not LVGL's defaults.

### In the emulator or on a watch

The emulator comes from [cmf-watch-utils](https://github.com/arnav-kr/cmf-watch-utils), which isn't part of this
repository: set `CMF_WATCH_UTILS` to a checkout with its `runtime/`, `fixtures/` and the stock firmware dump, and
`STOCK_NOR` to a stock NOR image. Installing into the emulator also needs Android's `netsimd` (`WP2_NETSIMD`).

```bash
make emu HCI_PORT=7000                  # stub firmware + App Loader, booted in the emulator
make run APP=my-app HCI_PORT=7000       # build, install the loader if missing, install the app
make list HCI_PORT=7000                 # what's installed; make remove SLOT=N HCI_PORT=7000
make run APP=my-app ADDRESS=<watch>     # the same on your own watch (address from `make scan`)
```

Everything you install on your watch is at your own risk.

## Publish it

Add two files next to `Cargo.toml`:

`store.toml`, the listing:

```toml
name = "My App"                       # the store's title, at most 60 characters
summary = "What it does, in one line" # at most 140
version = "0.1.0"                     # MAJOR.MINOR.PATCH, each part at most 999
author = "Your Name"                  # at most 60
published = true                      # false: uploaded, but hidden from the store
screenshots = ["shots/main.png"]      # optional, at most 8; png, jpeg, webp or svg, 2 MB each
```

`store.md`, the app page: Markdown, at most 8000 characters. Say what the app does, then what it needs under
`## Requirements` (the patched firmware, the App Loader, a companion app). Links like `/firmware` point into the store.

`make check` validates both. Open a pull request; [CONTRIBUTING.md](CONTRIBUTING.md) says what a review looks at.

### How an update reaches the store

Only through a pull request that raises the app's `version`:

1. **Pull request.** CI fails if an app's files changed but its `version` didn't go up from `main` (a version never
   goes down; a new app can start anywhere). It builds and tests every app the PR touches and attaches the packages
   to the run as the `store-apps` artifact; try yours before asking for a review.
2. **Merge to `main`.** CI publishes exactly the apps whose `version` went up (or that are new). Nothing else
   publishes: not a rebuild, not an SDK bump, not a `store.md` edit without a new version.
3. **The store** refuses a version lower than the one it has, and a new package under the same version.

The `.wp2app` header's version number is derived from `version` (`major << 20 | minor << 10 | patch`). Everything in `store.toml`, `store.md`, the icon and screenshots is the listing: a
field or file removed here is removed from the store with the next version.

## Maintainers

- **SDK bump:** change `WP2_APPS_REV` in the `Makefile`, `make build && make test`, open a PR. CI rebuilds every app
  but publishes none; raise an app's version in the same PR to ship it rebuilt.
- **Publishing by hand:** Actions → Store → Run workflow, with the apps and a dry run (the default), which asks the
  store what would change and saves nothing.
- The publish job sends each package to the store's `POST /api/store/publish` with the `STORE_PUBLISH_TOKEN` secret;
  `STORE_URL` is a repository variable. The token can only create or update app listings.

## License

MIT. Contributions are accepted under the same license.
