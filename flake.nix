{
  description = "xiu - A pure declarative Wayland desktop shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs supportedSystems (system: f system);
    in
    {
      # NixOS module for system-wide declarative configuration
      nixosModules.xiu = import ./nix/nixos.nix;
      nixosModules.default = self.nixosModules.xiu;

      # Home Manager module for user-level declarative configuration
      homeManagerModules.xiu = import ./nix/home-manager.nix;
      homeManagerModules.default = self.homeManagerModules.xiu;

      # Packages
      packages = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          xiu = pkgs.rustPlatform.buildRustPackage {
            pname = "xiu";
            version = "0.1.0";
            src = ./cli;
            cargoLock = {
              lockFile = ./cli/Cargo.lock;
            };
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [ pkgs.openssl ];
            doCheck = false;
          };
          default = self.packages.${system}.xiu;
        }
      );

      # Complete declarative NixOS configuration with Quick Setup defaults
      nixosConfigurations.xiu = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [
          self.nixosModules.default
          ./nix/configuration.nix
        ];
      };
    };
}
