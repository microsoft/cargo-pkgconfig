use clap::{App, AppSettings, Arg};
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

use path_slash::PathBufExt;

fn main() {
    let matches = App::new("cargo")
        .bin_name("cargo")
        .setting(AppSettings::TrailingVarArg)
        .setting(AppSettings::SubcommandRequired)
        .version(env!("CARGO_PKG_VERSION"))
        .author("Justin Moore <jusmoore@microsoft.com>")
        .about("Extract crate metadata with an interface similar to pkg-config")
        .subcommand(
            clap::app_from_crate!().name("pkgconfig").args(&[
                Arg::new("libname")
                    .help("Name of the library (usually the same as the crate name)")
                    .takes_value(true)
                    .required(true),
                Arg::new("libs")
                    .help("output all linker flags")
                    .long("libs")
                    .required(false),
                Arg::new("msvc")
                    .help("emit MSVC-style flags")
                    .long("msvc")
                    .required(false),
                Arg::new("cargocmd")
                    .takes_value(true)
                    .multiple_values(true)
                    .last(true),
            ]),
        )
        .get_matches();

    let matches = match matches.subcommand() {
        Some(("pkgconfig", matches)) => matches,
        _ => unreachable!(),
    };

    let name = matches.value_of("libname").unwrap();
    let dump_libs = matches.is_present("libs");
    let msvc = matches.is_present("msvc");

    let cargo_args = matches
        .values_of("cargocmd")
        .unwrap_or(clap::Values::default())
        .collect::<Vec<_>>();

    let cargo_args = ["build", "--message-format=json-render-diagnostics"]
        .into_iter()
        .chain(cargo_args.into_iter())
        .collect::<Vec<_>>();

    // Print out the cargo command line.
    // eprintln!("cargo {}", cargo_args.join(" "));

    // Invoke cargo and capture metadata.
    let mut command = Command::new("cargo")
        .args(&cargo_args)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let reader = std::io::BufReader::new(command.stdout.take().unwrap());
    for message in cargo_metadata::Message::parse_stream(reader) {
        match message.unwrap() {
            cargo_metadata::Message::CompilerArtifact(artifact) => {
                let target = &artifact.target;

                if target.name == name {
                    // Found the artifact we want.
                    // Check if it's either a lib or staticlib artifact.
                    if target
                        .kind
                        .iter()
                        .any(|s| ["lib", "staticlib"].iter().any(|t| t == s))
                        && dump_libs
                    {
                        // Determine path and dump library filename.
                        let filenames = &artifact.filenames;

                        // Find the first filename that matches one of *.lib, *.a, *.rlib.
                        // Whew, iterators are pretty awesome.
                        let filename = filenames.iter().find(|f| {
                            ["lib", "a", "rlib"]
                                .iter()
                                .any(|e| f.extension() == Some(e))
                        });

                        let filename = match filename {
                            Some(f) => f,
                            None => {
                                eprintln!("Found artifact \"{name}\", but did not find library artifact from {filenames:?}!");
                                continue;
                            }
                        };

                        // Determine the path to the artifact.
                        let filepath = filename.parent().unwrap();
                        let filename = filename.file_name().unwrap();

                        // HACK: Solely use forward slashes to ensure compatibility with Linux-based tooling.
                        let filepath = PathBuf::from(filepath).to_slash().unwrap();

                        if msvc {
                            // These additional libraries are typically required by Rust.
                            // FIXME: Figure out when, and why?
                            const ADDITIONAL_LIBS: &str = "Bcrypt.lib Userenv.lib";

                            println!("/LIBPATH:{filepath} {filename} {ADDITIONAL_LIBS}");
                        } else {
                            println!("-L{filepath} -l{name}");
                        }
                    }
                }
            }
            cargo_metadata::Message::BuildFinished(_) => {}
            _ => {}
        }
    }

    // TODO: Print an error if we didn't find the library the user specified on the command line.

    let output = command.wait().unwrap();

    std::process::exit(if let Some(code) = output.code() {
        code
    } else {
        1
    });
}
