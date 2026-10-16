# Entry point exposing xiu NixOS and Home Manager modules, deploy-set, and configuration
{
  nixos = import ./nixos.nix;
  homeManager = import ./home-manager.nix;
  deploySet = import ./deploy-set.nix;
  configuration = import ./configuration.nix;
}
