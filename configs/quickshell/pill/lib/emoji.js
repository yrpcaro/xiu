// The everyday glyphs plus the composite classics — one table shared in
// spirit with the xiu CLI's emoji command (cli/src/commands/emoji.rs).
// Pairs of [name, text]; a query matches when it appears in the name.
// Loaded with `.pragma library` semantics: plain data, no imports.
var EMOJI = [
    ["shrug", "¯\\_(ツ)_/¯"],
    ["tableflip", "(╯°□°)╯︵ ┻━┻"],
    ["unflip", "┬─┬ ノ( ゜-゜ノ)"],
    ["lol", "😂"],
    ["joy", "😂"],
    ["smile", "🙂"],
    ["grin", "😀"],
    ["wink", "😉"],
    ["thinking", "🤔"],
    ["shrugging", "🤷"],
    ["ok", "👌"],
    ["thumbsup", "👍"],
    ["thumbsdown", "👎"],
    ["clap", "👏"],
    ["wave", "👋"],
    ["pray", "🙏"],
    ["muscle", "💪"],
    ["point", "👉"],
    ["eyes", "👀"],
    ["heart", "❤️"],
    ["sparkle", "✨"],
    ["fire", "🔥"],
    ["star", "⭐"],
    ["boom", "💥"],
    ["100", "💯"],
    ["check", "✅"],
    ["cross", "❌"],
    ["question", "❓"],
    ["warning", "⚠️"],
    ["bulb", "💡"],
    ["rocket", "🚀"],
    ["party", "🎉"],
    ["cake", "🎂"],
    ["coffee", "☕"],
    ["pizza", "🍕"],
    ["beer", "🍺"],
    ["moon", "🌙"],
    ["sun", "☀️"],
    ["zap", "⚡"],
    ["snowflake", "❄️"],
    ["bug", "🐛"],
    ["skull", "💀"],
    ["ghost", "👻"],
    ["alien", "👽"],
    ["robot", "🤖"],
    ["cat", "🐱"],
    ["dog", "🐶"],
    ["fox", "🦊"],
    ["panda", "🐼"],
    ["poop", "💩"],
];

function matches(query) {
    var q = query.toLowerCase();
    return EMOJI.filter(function(e) { return e[0].indexOf(q) >= 0; });
}

