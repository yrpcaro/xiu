# /etc/nixos/configuration.nix for xiu
# Declarative configuration based on xiu Quick Setup defaults:
# - Terminal: foot
# - Login shell: fish with $EDITOR=helix
# - Editor: helix
# - File manager: dolphin (and full compatibility suite for yazi)
# - Browser: brave
# - Media player: mpv
# - Greeter / Display manager: direct TTY login (none) with Hyprland session (and optional SDDM with washi theme)
# - Keyboard layout: us,ir with option grp:alt_shift_toggle and variant ,winkeys
# - Touchpad: natural_scroll = true, tap_to_click = true
# - Idle timeout: 5m screen lock, 6m display off
# - Theming: Spicetify enabled, Telegram xiu theme enabled, GTK 3/4 & Qt Darkly styling
{ config, pkgs, lib, ... }:

{
  imports = [
    # Include hardware configuration if present
    ./hardware-configuration.nix
    ./nixos.nix
  ];

  # Bootloader
  boot.loader.systemd-boot.enable = lib.mkDefault true;
  boot.loader.efi.canTouchEfiVariables = lib.mkDefault true;

  # Networking
  networking.hostName = lib.mkDefault "xiu-nixos";
  networking.networkmanager.enable = true;

  # Time zone and locale
  time.timeZone = lib.mkDefault "UTC";
  i18n.defaultLocale = lib.mkDefault "en_US.UTF-8";

  # Enable declarative xiu configuration with Quick Setup defaults
  programs.xiu = {
    enable = true;
    defaultApps = {
      terminal = "foot";
      editor = "helix";
      fileManager = "dolphin";
      browser = "brave";
      mediaPlayer = "mpv";
    };
    greeter = "none"; # direct TTY login (none); switch to "sddm" for SDDM with washi theme
    sddmTheme = "washi";
    keyboardLayout = {
      layout = "us,ir";
      options = "grp:alt_shift_toggle";
      variant = ",winkeys";
    };
    touchpad = {
      naturalScroll = true;
      tapToClick = true;
    };
    idle = {
      lockTimeout = 300; # 5m screen lock
      dpmsTimeout = 360; # 6m display off
    };
    yaziSuite.enable = true;
    spicetify.enable = true;
    telegram.enable = true;
    theming.enable = true;
  };

  # Default user account with fish shell
  users.users.xiu = {
    isNormalUser = true;
    description = "xiu";
    extraGroups = [ "networkmanager" "wheel" "video" "audio" "input" ];
    shell = pkgs.fish;
  };

  system.stateVersion = "24.11";
}
