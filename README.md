# Now Playing for the COSMIC panel

A rewrite of [cosmic-ext-applet-dynamic-island](https://github.com/DrakRosmann/cosmic-ext-applet-dynamic-island),
laid out like the applets in [pop-os/cosmic-applets](https://github.com/pop-os/cosmic-applets).

In the panel, the applet shows the most important thing going on:

- **Media**: the cover, the title and bars that move in your accent color while it plays.
  It works with any MPRIS player (Spotify, browsers, VLC, mpv, Rhythmbox…). A paused player
  leaves the applet after 5 minutes.
- **Recording**: when an app records from the microphone, a red dot and how long it has been recording.
- **Charging**: when you plug in the charger, "Charging" and the percentage, for a moment.

With nothing going on, it shows only its icon, like the other applets.
Middle-click the applet to play or pause.

## The popup

Clicking the applet opens a Now Playing card like the one in the macOS menu bar:

- a large cover on the left (click it to bring up the player's window)
- the title, with the bars beside it, and the artist under it
- previous, play/pause and next, flat, with play/pause larger
- a progress bar you can drag, with the time played and the time left under it (`01:07`, `−02:59`)

With more than one player open, they are listed under the card to pick from. Apps recording
and the battery show below it only while they are active. **Customize**, at the bottom, picks
what the panel shows (the cover, the title and its maximum width, the bars) and which activities
the applet follows.

## What changed from the original

- No timer: the countdown, its presets, the "time's up" alert and its notification are gone.
- The popup is the Now Playing card above instead of a list.
- Built like cosmic-applets: a library with `run()`, Fluent translations through `i18n-embed`,
  settings through `CosmicConfigEntry` watched with `Core::watch_config`, the popup opened with
  `surface::action::app_popup` and placed under the applet with a rectangle tracker, a `justfile`
  and a `flake.nix` like theirs.
- Pinned to the same libcosmic revision, zbus 6 and patches as cosmic-applets.
- App ID `dev.cosmic_ext.CosmicAppletNowPlaying`. To change it, edit `APP_ID` in `src/lib.rs`,
  `appid` in the `justfile`, and rename the files in `data/`.

## Install

You need Rust 1.95 or newer, `just`, and libcosmic's build dependencies (`libxkbcommon`,
`wayland`, `libEGL`, `pkg-config`). At runtime, `pactl` is used to see apps recording and `curl`
to fetch covers players give as web links.

```sh
just                       # build
sudo just install          # into /usr
just prefix=~/.local install   # or for your user only
```

Then add **Now Playing** in Settings > Desktop > Panel > Applets, best in the middle of the top panel.

### NixOS

```sh
nix build        # ./result/bin/cosmic-applet-now-playing
nix develop      # a shell with everything to build it
```

Or in your system flake:

```nix
inputs.now-playing.url = "path:/path/to/cosmic-applet-now-playing";
# ...
environment.systemPackages = [ inputs.now-playing.packages.${pkgs.system}.default ];
```

The package wraps the program with `pactl` and `curl` on its `PATH`.

## Development

`just run` starts the applet with its popup open. `COSMIC_NOW_PLAYING_PREVIEW=popup` opens the popup
by itself when it starts, and `=settings` opens it on Customize, to take screenshots without
clicking the panel. `just check` runs clippy.

## License

GPL-3.0-only. Provided as is, without warranty: use it at your own risk.
