{
  description = "tracker-rs: A Renoise-inspired TUI music tracker with live coding, using Rust.";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};
    in {
      devShell = pkgs.mkShell {
        buildInputs = with pkgs; [
          rustup
          rustc
          cargo
          cargo-audit
          (if pkgs.stdenv.isLinux then [ pkgs.alsa-lib ] else [])  # Conditional ALSA inclusion for Linux
        ];

        shellHook = ''
          echo "Entering tracker-rs development environment!"
          rustup override set stable
          cargo fmt --all
          cargo check
        '';
      };

      defaultPackage = pkgs.rustPlatform.buildRustPackage {
        pname = "tracker-rs";
        version = "0.1.0";
        src = self;
        cargoSha256 = "sha256-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"; # Temporary dummy value

        buildInputs = with pkgs; [
          rustc
          cargo
          (if pkgs.stdenv.isLinux then [ pkgs.alsa-lib ] else [])  # Conditional ALSA inclusion
        ];

        meta = with pkgs.lib; {
          description = "Tracker: Renoise-inspired music app with TUI and live-coding features.";
          platforms = platforms.linux ++ platforms.darwin ++ platforms.windows;
        };
      };

    });
}