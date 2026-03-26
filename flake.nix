{
  description = "riffl: A Renoise-inspired TUI music tracker with live coding, using Rust.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};
        inherit (pkgs) lib stdenv;

        commonInputs = with pkgs;
          [
            libiconv
          ]
          ++ lib.optionals stdenv.isLinux [alsa-lib];

        devTools = with pkgs; [
          rustup
        ];
      in {
        # The environment for 'nix develop' or 'direnv'
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [pkg-config];

          buildInputs = commonInputs ++ devTools;

          shellHook = ''
                          echo "Entering riffl development environment!"
            # Ensure the toolchain is ready
                          rustup override set stable 2>/dev/null || true
                          rustup default stable
                          rustup component add rustfmt
          '';
        };

        # The recipe for 'nix build'
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "riffl";
          version = "0.1.0";
          src = self;

          # This must be updated whenever Cargo.toml/Cargo.lock changes
          cargoHash = "sha256-Mm4xjrZEZoiwuz/bKSg/46cWGfGq54lK1nePNT6tBxo=";

          buildInputs = commonInputs;

          # Often needed for crates that use bindgen or link to C libs
          nativeBuildInputs = with pkgs; [pkg-config];

          meta = with lib; {
            description = "Tracker: Renoise-inspired music app with TUI.";
            platforms = platforms.linux ++ platforms.darwin;
          };
        };
      }
    );
}
