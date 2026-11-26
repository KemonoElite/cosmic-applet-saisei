# Laid out as pop-os/cosmic-applets' justfile.
name := 'cosmic-applet-now-playing'
appid := 'dev.cosmic_ext.CosmicAppletNowPlaying'

rootdir := ''
prefix := '/usr'
debug := '0'
vendor := '0'
target := if debug == '1' { 'debug' } else { 'release' }
vendor_args := if vendor == '1' { '--frozen --offline' } else { '' }
debug_args := if debug == '1' { '' } else { '--release' }
cargo_args := vendor_args + ' ' + debug_args
targetdir := env('CARGO_TARGET_DIR', 'target')

sharedir := rootdir + prefix + '/share'
iconsdir := sharedir + '/icons/hicolor'
bindir := rootdir + prefix + '/bin'

default: build-release

# Compiles with debug profile
build-debug *args:
    cargo build {{ args }}

# Compiles with release profile
build-release *args: (build-debug '--release' args)

# Compile with a vendored tarball
build-vendored *args: vendor-extract (build-release '--frozen --offline' args)

# Runs clippy, as cosmic-applets' CI does
check *args:
    cargo clippy --all-features {{ args }}

# Runs the applet outside the panel, opening its popup by itself
run *args:
    COSMIC_NOW_PLAYING_PREVIEW=popup RUST_LOG=cosmic_applet_now_playing=debug cargo run {{ args }}

[private]
_install_bin:
    install -Dm0755 {{ targetdir }}/{{ target }}/{{ name }} {{ bindir }}/{{ name }}

# Installs the desktop entry, metainfo and icons, without the program
install-data:
    install -Dm0644 data/{{ appid }}.desktop {{ sharedir }}/applications/{{ appid }}.desktop
    install -Dm0644 data/{{ appid }}.metainfo.xml {{ sharedir }}/metainfo/{{ appid }}.metainfo.xml
    find data/icons -type f -exec echo {} \; | rev | cut -d'/' -f-3 | rev | xargs -d '\n' -I {} install -Dm0644 data/icons/{} {{ iconsdir }}/{}

# Installs files into the system
install: _install_bin install-data

# Uninstalls installed files
uninstall:
    rm -f {{ bindir }}/{{ name }}
    rm -f {{ sharedir }}/applications/{{ appid }}.desktop
    rm -f {{ sharedir }}/metainfo/{{ appid }}.metainfo.xml
    find data/icons -type f -exec echo {} \; | rev | cut -d'/' -f-3 | rev | xargs -d '\n' -I {} rm -f {{ iconsdir }}/{}

# Vendor Cargo dependencies locally
vendor:
    mkdir -p .cargo
    cargo vendor --locked | head -n -1 > .cargo/config.toml
    echo 'directory = "vendor"' >> .cargo/config.toml
    tar pcf vendor.tar vendor
    rm -rf vendor

# Extracts vendored dependencies
[private]
vendor-extract:
    rm -rf vendor
    tar pxf vendor.tar
