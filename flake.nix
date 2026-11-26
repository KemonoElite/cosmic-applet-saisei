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

  # Laid out as pop-os/cosmic-applets' flake, plus a NixOS module.
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
            just prefix=$out install-data
            # pactl sees apps recording; curl fetches covers players give as links.
            wrapProgram $out/bin/cosmic-applet-now-playing \
              --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.pulseaudio pkgs.curl ]}
          '';
          meta = {
            description = "What's playing, apps recording and charging, in the COSMIC panel";
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
        packages.cosmic-applet-now-playing = now-playing;

        apps.default = flake-utils.lib.mkApp {
          drv = now-playing;
        };

        devShells.default = pkgs.mkShell rec {
          inputsFrom = builtins.attrValues self.checks.${system};
          packages = with pkgs; [ rust-analyzer pulseaudio ];
          LD_LIBRARY_PATH = pkgs.lib.strings.makeLibraryPath (builtins.concatMap (d: d.runtimeDependencies) inputsFrom);
        };
      })
    // {
      # programs.cosmic-applet-now-playing.enable = true;
      nixosModules.default = self.nixosModules.cosmic-applet-now-playing;
      nixosModules.cosmic-applet-now-playing = { config, lib, pkgs, ... }:
        let
          cfg = config.programs.cosmic-applet-now-playing;
        in
        {
          options.programs.cosmic-applet-now-playing = {
            enable = lib.mkEnableOption "the Now Playing applet for the COSMIC panel";

            package = lib.mkOption {
              type = lib.types.package;
              default = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
              defaultText = lib.literalExpression "cosmic-applet-now-playing.packages.\${pkgs.stdenv.hostPlatform.system}.default";
              description = "The Now Playing applet package to install.";
            };
          };

          # The panel finds applets by their desktop entries in
          # /run/current-system/sw/share/applications.
          config = lib.mkIf cfg.enable {
            environment.systemPackages = [ cfg.package ];
          };
        };
    };

  nixConfig = {
    # Cache for the Rust toolchain in fenix
    extra-substituters = [ "https://nix-community.cachix.org" ];
    extra-trusted-public-keys = [ "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs=" ];
  };
}
