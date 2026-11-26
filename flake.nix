{
  description = "Now Playing: what's playing, apps recording and charging, in the COSMIC panel";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    nix-filter.url = "github:numtide/nix-filter";
    crane.url = "github:ipetkov/crane";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  # Laid out as pop-os/cosmic-applets' flake.
  outputs = { self, nixpkgs, flake-utils, nix-filter, crane, fenix }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        craneLib = (crane.mkLib pkgs).overrideToolchain fenix.packages.${system}.stable.toolchain;

        pkgDef = {
          pname = "cosmic-applet-now-playing";
          version = "0.2.0";
          src = nix-filter.lib.filter {
            root = ./.;
            exclude = [
              ./.gitignore
              ./flake.nix
              ./flake.lock
              ./LICENSE
              ./README.md
            ];
          };
          nativeBuildInputs = with pkgs; [ just pkg-config autoPatchelfHook makeWrapper ];
          buildInputs = with pkgs; [
            libxkbcommon
            libglvnd # For libEGL
          ];
          runtimeDependencies = with pkgs; [ wayland libglvnd ];
        };

        cargoArtifacts = craneLib.buildDepsOnly pkgDef;

        now-playing = craneLib.buildPackage (pkgDef // {
          inherit cargoArtifacts;
          postInstall = ''
            just rootdir= prefix=$out install-data
            # pactl sees apps recording; curl fetches covers players give as links.
            wrapProgram $out/bin/cosmic-applet-now-playing \
              --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.pulseaudio pkgs.curl ]}
          '';
          meta = {
            description = "Live activities for the COSMIC panel";
            license = pkgs.lib.licenses.gpl3Only;
            mainProgram = "cosmic-applet-now-playing";
            platforms = pkgs.lib.platforms.linux;
          };
        });
      in {
        checks = {
          inherit now-playing;
        };

        packages.default = now-playing;

        apps.default = flake-utils.lib.mkApp {
          drv = now-playing;
        };

        devShells.default = pkgs.mkShell rec {
          inputsFrom = builtins.attrValues self.checks.${system};
          packages = with pkgs; [ rust-analyzer pulseaudio ];
          LD_LIBRARY_PATH = pkgs.lib.strings.makeLibraryPath (builtins.concatMap (d: d.runtimeDependencies) inputsFrom);
        };
      });

  nixConfig = {
    # Cache for the Rust toolchain in fenix
    extra-substituters = [ "https://nix-community.cachix.org" ];
    extra-trusted-public-keys = [ "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs=" ];
  };
}
