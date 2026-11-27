# Saisei for the COSMIC panel

*Saisei* (再生) is Japanese for "playback", the word on Japanese play buttons.

A rewrite of [cosmic-ext-applet-dynamic-island](https://github.com/DrakRosmann/cosmic-ext-applet-dynamic-island),
laid out like the applets in [pop-os/cosmic-applets](https://github.com/pop-os/cosmic-applets).

In the panel, the applet shows what's playing: the cover, the title and bars that move in your
accent color while it plays. It works with any MPRIS player (Spotify, browsers, VLC, mpv,
Rhythmbox…). A paused player leaves the applet after 5 minutes.

With nothing playing, it shows only its icon, like the other applets.
Middle-click the applet to play or pause.

## The popup

Clicking the applet opens a card like Now Playing in the macOS menu bar:

- a large cover on the left (click it to bring up the player's window)
- the title and the artist, each scrolling when it is too long to fit
- the bars and a small toggle switch in the top-right corner
- previous, play/pause and next, flat, with play/pause larger
- a progress bar you can drag, with the time played and the time left under it (`01:07`, `−02:59`)

With more than one player open, they are listed under the card to pick from. The toggle opens
**Customize**, which turns the cover, the title and the bars on the panel on or off.

## What changed from the original

- No timer: the countdown, its presets, the "time's up" alert and its notification are gone.
- The popup is the card above instead of a list, with Customize behind an icon in its
  corner instead of a row of its own.
- Covers stay put while browsers replace their cover files, instead of flickering.
- Built like cosmic-applets: a library with `run()`, Fluent translations through `i18n-embed`,
  settings through `CosmicConfigEntry` watched with `Core::watch_config`, the popup opened with
  `surface::action::app_popup` and placed under the applet with a rectangle tracker, a `justfile`
  and a `flake.nix` like theirs.
- Pinned to the same libcosmic revision, zbus 6 and patches as cosmic-applets.
- App ID `dev.cosmic_ext.CosmicAppletSaisei`. To change it, edit `APP_ID` in `src/lib.rs`,
  `appid` in the `justfile`, and rename the files in `data/`.

## Install

You need Rust 1.95 or newer, `just`, and libcosmic's build dependencies (`libxkbcommon`,
`wayland`, `libEGL`, `pkg-config`). At runtime, `curl` is used
to fetch covers players give as web links.

```sh
just                       # build
sudo just install          # into /usr
just prefix=~/.local install   # or for your user only
```

Then add **Saisei** in Settings > Desktop > Panel > Applets, best in the middle of the top panel.

### NixOS

```sh
nix build        # ./result/bin/cosmic-applet-saisei
nix develop      # a shell with everything to build it
```

Or in your system flake:

```nix
inputs.cosmic-applet-saisei = {
  url = "github:KemonoElite/cosmic-applet-saisei";
  inputs.nixpkgs.follows = "nixpkgs";
};

# in the host's modules:
inputs.cosmic-applet-saisei.nixosModules.default
{ programs.cosmic-applet-saisei.enable = true; }
```

The package wraps the program with `curl` on its `PATH`.

## Development

`just run` starts the applet with its popup open. `COSMIC_SAISEI_PREVIEW=popup` opens the popup
by itself when it starts, and `=settings` opens it on Customize, to take screenshots without
clicking the panel. `just check` runs clippy.

## License

GPL-3.0-only. Provided as is, without warranty: use it at your own risk.
