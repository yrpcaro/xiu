# xiu NixOS system module: declaratively manage system-level xiu desktop
# environment and session components with Quick Setup defaults:
# - Terminal: foot
# - Login shell: fish with $EDITOR=helix
# - Editor: helix
# - File manager: dolphin (and full yazi companion suite)
# - Browser: brave
# - Media player: mpv
# - Greeter / Display manager: direct TTY login (none) with Hyprland session (and optional SDDM with washi theme)
# - Keyboard layout: us,ir with option grp:alt_shift_toggle and variant ,winkeys
# - Touchpad: natural_scroll = true, tap_to_click = true
# - Idle timeout: 5m screen lock, 6m display off
# - Theming & Integrations: Spicetify enabled, Telegram xiu theme enabled, GTK 3/4 & Qt Darkly styling
{ config, lib, pkgs, ... }:

with lib;
let
  cfg = config.programs.xiu;

  yaziSuitePackages = with pkgs; [
    yazi
    ripdrag
    trash-cli
    p7zip
    ffmpegthumbnailer
    poppler_utils
    resvg
    chafa
    lazygit
    simple-mtpfs
  ];

  xiuSystemPackages = with pkgs; [
    # Core applications from Quick Setup
    foot
    helix
    kdePackages.dolphin
    brave
    mpv

    # Desktop & rice utilities
    matugen
    swww
    hyprpicker
    hyprpolkitagent
    libsecret
    fastfetch
    btop
    cava
    micro
    htop
    neovim
    bottom
    zathura
    tealdeer
    navi
    surge
    mangofetch
    uwsm

    # Theming & assets
    bibata-cursors
    adwaita-icon-theme
    kdePackages.breeze-icons
  ] ++ (optionals cfg.yaziSuite.enable yaziSuitePackages)
    ++ (optional cfg.spicetify.enable pkgs.spicetify-cli)
    ++ (optional cfg.telegram.enable pkgs.telegram-desktop);
in
{
  options.programs.xiu = {
    enable = mkEnableOption "the xiu declarative desktop environment on NixOS";

    defaultApps = {
      terminal = mkOption {
        type = types.str;
        default = "foot";
        description = "Default terminal package/command.";
      };
      editor = mkOption {
        type = types.str;
        default = "helix";
        description = "Default editor package/command ($EDITOR).";
      };
      fileManager = mkOption {
        type = types.str;
        default = "dolphin";
        description = "Default file manager.";
      };
      browser = mkOption {
        type = types.str;
        default = "brave";
        description = "Default web browser.";
      };
      mediaPlayer = mkOption {
        type = types.str;
        default = "mpv";
        description = "Default media player.";
      };
    };

    greeter = mkOption {
      type = types.enum [ "none" "sddm" ];
      default = "none";
      description = "Greeter / display manager. 'none' provides direct TTY login with Hyprland session; 'sddm' uses SDDM display manager.";
    };

    sddmTheme = mkOption {
      type = types.enum [ "washi" "torii" ];
      default = "washi";
      description = "SDDM login screen theme.";
    };

    keyboardLayout = {
      layout = mkOption {
        type = types.str;
        default = "us,ir";
        description = "XKB keyboard layout list.";
      };
      options = mkOption {
        type = types.str;
        default = "grp:alt_shift_toggle";
        description = "XKB keyboard layout switching options.";
      };
      variant = mkOption {
        type = types.str;
        default = ",winkeys";
        description = "XKB keyboard variant list.";
      };
    };

    touchpad = {
      naturalScroll = mkOption {
        type = types.bool;
        default = true;
        description = "Enable natural scrolling on touchpad.";
      };
      tapToClick = mkOption {
        type = types.bool;
        default = true;
        description = "Enable tap-to-click on touchpad.";
      };
    };

    idle = {
      lockTimeout = mkOption {
        type = types.int;
        default = 300; # 5 minutes
        description = "Idle screen lock timeout in seconds.";
      };
      dpmsTimeout = mkOption {
        type = types.int;
        default = 360; # 6 minutes
        description = "Idle display power off timeout in seconds.";
      };
    };

    yaziSuite = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Install and enable full yazi companion suite (termfilechooser, ripdrag, trash-cli, 7zip, ffmpegthumbnailer, poppler, resvg, chafa, lazygit, simple-mtpfs).";
      };
    };

    theming = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Enable GTK 3/4 and Qt Darkly styling across the system.";
      };
      style = mkOption {
        type = types.str;
        default = "Darkly";
        description = "Qt style theme.";
      };
    };

    spicetify = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Enable Spicetify CLI.";
      };
    };

    telegram = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Enable Telegram desktop with xiu palette styling.";
      };
    };

    loginShell = mkOption {
      type = types.enum [ "fish" "zsh" "bash" ];
      default = "fish";
      description = "Default login shell.";
    };
  };

  config = mkIf cfg.enable {
    # Wayland compositor: Hyprland with UWSM session support
    programs.hyprland = {
      enable = true;
      withUWSM = true;
    };

    # Login Shell: fish with $EDITOR set to helix
    programs.fish = mkIf (cfg.loginShell == "fish") {
      enable = true;
    };

    # System-wide environment variables
    environment.variables = {
      EDITOR = cfg.defaultApps.editor;
      VISUAL = cfg.defaultApps.editor;
      QT_QPA_PLATFORMTHEME = "kde";
      QT_STYLE_OVERRIDE = cfg.theming.style;
      QT_USE_PORTAL = "1";
      GTK_USE_PORTAL = "1";
    };

    # Display manager / Greeter: direct TTY login (none) or SDDM with washi theme
    services.displayManager.sddm = mkIf (cfg.greeter == "sddm") {
      enable = true;
      wayland.enable = true;
      theme = cfg.sddmTheme;
    };

    # Keyboard layout: us,ir with grp:alt_shift_toggle and ,winkeys
    services.xserver.xkb = {
      layout = cfg.keyboardLayout.layout;
      options = cfg.keyboardLayout.options;
      variant = cfg.keyboardLayout.variant;
    };

    # Touchpad: natural scrolling and tap-to-click
    services.libinput = {
      enable = true;
      touchpad = {
        naturalScrolling = cfg.touchpad.naturalScroll;
        tapping = cfg.touchpad.tapToClick;
      };
    };

    # Audio with Pipewire & Wireplumber
    services.pipewire = {
      enable = true;
      alsa.enable = true;
      alsa.support32Bit = true;
      pulse.enable = true;
    };

    # Security: Polkit & PAM hyprlock
    security.polkit.enable = true;
    security.pam.services.hyprlock = {};

    # Secret service: GNOME Keyring
    services.gnome.gnome-keyring.enable = true;

    # Portals: Hyprland + GTK backends
    xdg.portal = {
      enable = true;
      extraPortals = [
        pkgs.xdg-desktop-portal-hyprland
        pkgs.xdg-desktop-portal-gtk
      ];
      config.common.default = [ "hyprland" "gtk" ];
    };

    # Dconf & Qt styling
    programs.dconf.enable = true;
    qt = mkIf cfg.theming.enable {
      enable = true;
      platformTheme = "kde";
      style = cfg.theming.style;
    };

    # Fonts
    fonts.packages = with pkgs; [
      nerd-fonts.jetbrains-mono
      font-awesome
      noto-fonts
      noto-fonts-cjk-sans
      noto-fonts-emoji
    ];

    # System packages
    environment.systemPackages = xiuSystemPackages;
  };
}
