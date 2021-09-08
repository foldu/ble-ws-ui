{
  description = "A very basic flake";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    naersk = {
      url = "github:nmattia/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, naersk }: {
    overlay = final: prev: {
      ble-ws-gtk =
        let
          pkgs = nixpkgs.legacyPackages.${prev.system};
          cargoToml = builtins.fromTOML (builtins.readFile "${self}/Cargo.toml");
          version = cargoToml.package.version;
          pname = cargoToml.package.name;
        in
        pkgs.rustPlatform.buildRustPackage {
          src = self;
          inherit pname version;
          nativeBuildInputs = with pkgs; [
            pkg-config
            cmake
            wrapGAppsHook
          ];
          buildInputs = with pkgs; [
            glib
            gtk4
            libadwaita
          ];
          preFixup = ''
            mkdir -p "$out/share/applications"
            cp "$src"/*.desktop "$out/share/applications"
          '';
          cargoSha256 = "sha256-xMf/hzdBzFZxoRiMEIDtK0K0glii9HZmVrotm1FFcZk=";
          PROTOC = "${pkgs.protobuf}/bin/protoc";
          PROTOC_INCLUDE = "${pkgs.protobuf}/include";
        };
    };
  } // flake-utils.lib.eachDefaultSystem (
    system:
    let
      pkgs = import nixpkgs { inherit system; overlays = [ self.overlay ]; };
    in
    {
      defaultPackage = pkgs.ble-ws-gtk;
      packages = {
        inherit (pkgs) ble-ws-gtk;
      };
      devShell = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          gtk4
          glib
          pkg-config
          cmake
          libadwaita
        ];
        LD_LIBRARY_PATH = pkgs.lib.strings.makeLibraryPath (
          with pkgs; [
            glib
            gtk4
            libadwaita
            graphene
            fontconfig
            freetype
            cairo
          ]
        );
        RUSTFLAGS = "--cfg unsound_local_offset -C link-arg=-fuse-ld=lld";
        PROTOC = "${pkgs.protobuf}/bin/protoc";
        GSETTINGS_SCHEMA_DIR = "./data/schemas";
        PROTOC_INCLUDE = "${pkgs.protobuf}/include";
        TOKEN_FILE = "./token";
      };
    }
  );
}
