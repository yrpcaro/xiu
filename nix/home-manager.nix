# xiu for home-manager: manages xiu desktop configurations and user environment
# declaratively, with the Quick Setup defaults pre-configured:
# - Terminal: foot
# - Login shell: fish with $EDITOR=helix
# - Editor: helix
# - File manager: dolphin (and full compatibility suite for yazi)
# - Browser: brave
# - Media player: mpv
# - Keyboard layout: us,ir with option grp:alt_shift_toggle and variant ,winkeys
# - Touchpad: natural_scroll = true, tap_to_click = true
# - Idle timeout: 5m screen lock, 6m display off
# - Theming: Spicetify enabled, Telegram xiu theme enabled, GTK 3/4 & Qt Darkly styling
{ config, lib, pkgs, ... }:

with lib;
let
  cfg = config.programs.xiu;
  files = import ./deploy-set.nix { repo = cfg.repo; };
  wanted = filter (f: !(elem f.dest cfg.exclude)) files;

  # Idle configuration matching Quick Setup timeouts: 5m lock, 6m display off
  hypridleConf = ''
    general {
        lock_cmd = xiu lock
        before_sleep_cmd = loginctl lock-session
        after_sleep_cmd = hyprctl dispatch dpms on
    }

    listener {
        timeout = ${toString cfg.idle.lockTimeout}
        on-timeout = xiu lock --idle
    }

    listener {
        timeout = ${toString cfg.idle.dpmsTimeout}
        on-timeout = hyprctl dispatch dpms off
        on-resume = hyprctl dispatch dpms on
    }
  '';

  # Input configuration matching Quick Setup defaults
  inputLua = ''
    hl.config({
        input = {
            kb_layout          = "${cfg.input.kbLayout}",
            kb_variant         = "${cfg.input.kbVariant}",
            kb_options         = "${cfg.input.kbOptions}",
            follow_mouse       = 1,
            sensitivity        = 0,
            accel_profile      = "flat",
            repeat_rate        = 40,
            repeat_delay       = 400,
            numlock_by_default = false,
            touchpad = {
                natural_scroll = ${boolToString cfg.input.touchpad.naturalScroll},
                tap_to_click   = ${boolToString cfg.input.touchpad.tapToClick},
            },
        },
        cursor = {
            no_hardware_cursors = true,
        },
        binds = {
            scroll_event_delay = 40,
        },
    })
  '';

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

  quickSetupPackages = with pkgs; [
    # Core applications
    foot
    helix
    kdePackages.dolphin
    brave
    mpv

    # Rice utilities & tools
    matugen
    swww
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
    hyprpicker
    hyprpolkitagent
    bibata-cursors
    adwaita-icon-theme
  ] ++ (optionals cfg.yaziSuite.enable yaziSuitePackages)
    ++ (optional cfg.spicetify.enable pkgs.spicetify-cli)
    ++ (optional cfg.telegram.enable pkgs.telegram-desktop);
in
{
  options.programs.xiu = {
    enable = mkEnableOption "the xiu desktop environment and configurations";

    repo = mkOption {
      type = types.path;
      description = "Path to the xiu checkout the links point into.";
    };

    exclude = mkOption {
      type = types.listOf types.str;
      default = [ ];
      description = "Extra dest-relative files to leave alone, on top of the user-owned set.";
    };

    installPackages = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to install default applications and desktop tools.";
    };

    defaultApps = {
      terminal = mkOption {
        type = types.str;
        default = "foot";
        description = "Default terminal emulator.";
      };
      editor = mkOption {
        type = types.str;
        default = "helix";
        description = "Default text editor.";
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

    yaziSuite = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Full compatibility companion suite for yazi (termfilechooser, ripdrag, trash-cli, 7zip, ffmpegthumbnailer, poppler, resvg, chafa, lazygit, simple-mtpfs).";
      };
    };

    input = {
      kbLayout = mkOption {
        type = types.str;
        default = "us,ir";
        description = "Keyboard layout list.";
      };
      kbVariant = mkOption {
        type = types.str;
        default = ",winkeys";
        description = "Keyboard variant list.";
      };
      kbOptions = mkOption {
        type = types.str;
        default = "grp:alt_shift_toggle";
        description = "Keyboard layout switching option.";
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
    };

    idle = {
      lockTimeout = mkOption {
        type = types.int;
        default = 300; # 5 minutes
        description = "Idle timeout in seconds before locking the screen.";
      };
      dpmsTimeout = mkOption {
        type = types.int;
        default = 360; # 6 minutes
        description = "Idle timeout in seconds before turning displays off.";
      };
    };

    theming = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Enable GTK 3/4 and Qt Darkly styling.";
      };
      style = mkOption {
        type = types.str;
        default = "Darkly";
        description = "Qt style override.";
      };
    };

    spicetify = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Enable Spicetify Spotify integration.";
      };
    };

    telegram = {
      enable = mkOption {
        type = types.bool;
        default = true;
        description = "Enable Telegram xiu theme support.";
      };
    };

    manageUserConfigs = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to declaratively manage hypridle and input configurations.";
    };
  };

  config = mkIf cfg.enable {
    home.packages = mkIf cfg.installPackages quickSetupPackages;

    home.sessionVariables = {
      EDITOR = cfg.defaultApps.editor;
      VISUAL = cfg.defaultApps.editor;
      QT_QPA_PLATFORMTHEME = "kde";
      QT_STYLE_OVERRIDE = cfg.theming.style;
      QT_USE_PORTAL = "1";
      GTK_USE_PORTAL = "1";
      QS_ICON_THEME = "yet-another-monochrome-icon-set";
    };

    # Deploy symlinks
    xdg.configFile = listToAttrs
      (map (f: nameValuePair f.dest { source = f.src; }) wanted)
      // mkIf cfg.manageUserConfigs {
        "hypr/hypridle.conf".text = hypridleConf;
        "hypr/modules/input.lua".text = inputLua;
      };

    # Theming: GTK 3/4 & Qt Darkly styling
    gtk = mkIf cfg.theming.enable {
      enable = true;
      gtk3.extraConfig = {
        gtk-application-prefer-dark-theme = 1;
      };
      gtk4.extraConfig = {
        gtk-application-prefer-dark-theme = 1;
      };
    };

    qt = mkIf cfg.theming.enable {
      enable = true;
      platformTheme.name = "kde";
      style.name = cfg.theming.style;
    };
  };
}
