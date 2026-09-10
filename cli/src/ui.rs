//! The ember skin shared by the CLI's own output (status, update, uninstall):
//! a vermilion edge marker, a quiet connector gutter, the rice's palette.
//! Empty strings when stdout is not a terminal or NO_COLOR is set, so a pipe
//! gets clean plain text.

use std::io::IsTerminal;

pub struct Skin {
    pub verm: &'static str,
    pub flame: &'static str,
    pub cream: &'static str,
    pub bright: &'static str,
    pub dim: &'static str,
    pub faint: &'static str,
    pub rst: &'static str,
}

const PLAIN: Skin = Skin {
    verm: "",
    flame: "",
    cream: "",
    bright: "",
    dim: "",
    faint: "",
    rst: "",
};

pub fn skin() -> Skin {
    if !std::io::stdout().is_terminal() || std::env::var_os("NO_COLOR").is_some() {
        return PLAIN;
    }
    Skin {
        verm: "\x1b[38;2;192;68;43m",
        flame: "\x1b[38;2;255;154;100m",
        cream: "\x1b[38;2;230;214;203m",
        bright: "\x1b[38;2;255;246;240m",
        dim: "\x1b[38;2;138;125;116m",
        faint: "\x1b[38;2;111;99;91m",
        rst: "\x1b[0m",
    }
}

pub fn sec(k: &Skin, title: &str) {
    println!("\n  {}▌{} {}{}{}", k.verm, k.rst, k.cream, title, k.rst);
}

pub fn gap(k: &Skin) {
    println!("  {}▏{}", k.faint, k.rst);
}

pub fn row(k: &Skin, colored: &str) {
    println!("  {}▏{}  {}", k.faint, k.rst, colored);
}

pub fn act(k: &Skin, verb: &str, what: &str) {
    println!("  {}▫{} {}{}{} {}{}{}", k.flame, k.rst, k.cream, verb, k.rst, k.bright, what, k.rst);
}

pub fn note(k: &Skin, text: &str) {
    println!("  {}▫{} {}{}{}", k.faint, k.rst, k.dim, text, k.rst);
}

pub fn ctl_die(k: &Skin, text: &str) -> i32 {
    eprintln!("  {}▌{} {}{}{}", k.verm, k.rst, k.verm, text, k.rst);
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_skin_when_piped() {
        // Tests run with stdout captured, so the terminal probe reads false
        // — which is exactly the path under test.
        let k = skin();
        assert_eq!(k.verm, "");
        assert_eq!(k.rst, "");
    }

    #[test]
    fn plain_skin_when_no_color() {
        // NO_COLOR forces plain regardless of the terminal.
        std::env::set_var("NO_COLOR", "1");
        let k = skin();
        std::env::remove_var("NO_COLOR");
        assert_eq!(k.verm, "");
    }
}
