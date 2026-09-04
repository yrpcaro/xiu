# xiu for home-manager: config symlinks only, no build step, no store
# plumbing. Every deploy-set file becomes an xdg.configFile source link into
# your xiu checkout, except the files the rice treats as user-owned (Settings
# keybinds and layout, hypridle, fish) — those stay plain files so the shell's
# own Settings surface keeps working. Pass `exclude` for anything else you
# would rather own in place.
#
#   imports = [ /path/to/xiu/nix/home-manager.nix ];
#   programs.xiu.repo = /path/to/xiu;
{ config, lib, ... }:

with lib;
let
  cfg = config.programs.xiu;
  files = import ./deploy-set.nix { repo = cfg.repo; };
  wanted = filter (f: !(elem f.dest cfg.exclude)) files;
in
{
  options.programs.xiu = {
    enable = mkEnableOption "the xiu rice configs as symlinks";

    repo = mkOption {
      type = types.path;
      description = "Path to the xiu checkout the links point into.";
    };

    exclude = mkOption {
      type = types.listOf types.str;
      default = [ ];
      description = "Extra dest-relative files to leave alone, on top of the
user-owned set the module never touches.";
    };
  };

  config = mkIf cfg.enable {
    xdg.configFile = listToAttrs
      (map (f: nameValuePair f.dest { source = f.src; }) wanted);
  };
}
