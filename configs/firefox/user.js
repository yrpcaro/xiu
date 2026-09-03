// Xiu Firefox prefs. userChrome.css and the live-theme extension need these.
user_pref("toolkit.legacyUserProfileCustomizations.stylesheets", true);
// The bundled xiu live-theme extension is unsigned. Temporary load via
// about:debugging always works; permanent loading needs a dev build, ESR or
// a self-signed (AMO unlisted) build — enable at your own discretion:
// user_pref("xpinstall.signatures.required", false);
