// Xiu Zen prefs. userChrome.css and the live-theme extension need these.
user_pref("toolkit.legacyUserProfileCustomizations.stylesheets", true);
// The bundled xiu live-theme extension is unsigned. Temporary load via
// about:debugging always works; permanent loading needs a dev build or a
// self-signed build — enable at your own discretion:
// user_pref("xpinstall.signatures.required", false);
