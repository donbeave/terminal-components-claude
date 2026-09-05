//! `TablePro` binary entry point and terminal option parsing.

use tui_next::{ColorLevel, Theme};

struct Options {
    theme: Theme,
    connect: Option<String>,
}

fn parse_args() -> Result<Option<Options>, String> {
    let mut level = ColorLevel::detect();
    let mut theme_name = "junie";
    let mut connect = None;
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        let arg = arg.to_string_lossy();
        match arg.as_ref() {
            "--color" | "-c" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--color requires truecolor|256|16|none".to_owned())?;
                level = match value.to_string_lossy().as_ref() {
                    "truecolor" | "24bit" => ColorLevel::TrueColor,
                    "256" => ColorLevel::Ansi256,
                    "16" => ColorLevel::Ansi16,
                    "none" | "mono" => ColorLevel::Mono,
                    other => {
                        return Err(format!(
                            "unknown --color value {other:?}; use truecolor|256|16|none"
                        ));
                    }
                };
            }
            "--theme" | "-t" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--theme requires junie|paper".to_owned())?;
                let value = value.to_string_lossy();
                if !matches!(value.as_ref(), "junie" | "paper") {
                    return Err("unknown --theme value; use junie|paper".to_owned());
                }
                theme_name = if value == "paper" { "paper" } else { "junie" };
            }
            "--connect" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--connect requires a connection name".to_owned())?;
                connect = Some(value.to_string_lossy().into_owned());
            }
            "-h" | "--help" => return Ok(None),
            unknown if unknown.starts_with('-') => {
                return Err(format!("unknown option {unknown:?}"));
            }
            _ => {}
        }
    }
    let base = if theme_name == "paper" {
        Theme::paper()
    } else {
        Theme::junie()
    };
    Ok(Some(Options {
        theme: base.downgrade(level),
        connect,
    }))
}

fn print_help() {
    println!(
        "tablepro — terminal database workbench\n\n\
         USAGE: tablepro [--theme junie|paper] [--color truecolor|256|16|none] [--connect NAME]\n\n\
         Keys: Ctrl+O quick open · Ctrl+T new query · Ctrl+R run · Ctrl+Y history · ? help · Ctrl+Q quit"
    );
}

fn main() -> std::io::Result<()> {
    let Some(options) = parse_args().map_err(std::io::Error::other)? else {
        print_help();
        return Ok(());
    };
    tablepro_app::run_with(options.theme, options.connect.as_deref())
}
