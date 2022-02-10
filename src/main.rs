use clap::{App, AppSettings, Arg};
use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

use path_slash::PathBufExt;

/// Tool to dump commands for
enum DumpType {
    /// `ar` on Linux, `lib` on Windows.
    Archiver,
    /// `ld` on Linux, `link` on Windows.
    Linker,
}

/// Linker command line flavor
#[derive(PartialEq, Clone)]
enum LinkerFlavor {
    /// Microsoft Visual Studio
    MSVC,
    /// GNU Compiler Collection
    GCC,
}

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
                    .required(false)
                    .conflicts_with("ar"),
                Arg::new("ar")
                    .help("output all archiver flags")
                    .long("ar")
                    .required(false)
                    .conflicts_with("libs"),
                Arg::new("flavor")
                    .help("flavor of linker command-line flags")
                    .long("flavor")
                    .takes_value(true)
                    .possible_values(["msvc", "gcc"])
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
    let dump_type = {
        if matches.is_present("libs") {
            DumpType::Linker
        } else if matches.is_present("ar") {
            DumpType::Archiver
        } else {
            eprintln!("No output format specified!");
            std::process::exit(1);
        }
    };

    let flavor = match matches.value_of("flavor") {
        Some("msvc") => LinkerFlavor::MSVC,
        Some("gcc") => LinkerFlavor::GCC,
        _ => {
            // HACK: Use OS to extract style.
            // Really should use the target's compiler triple instead.
            if cfg!(target_os = "windows") {
                LinkerFlavor::MSVC
            } else {
                LinkerFlavor::GCC
            }
        }
    };

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

    let mut artifacts = Vec::new();

    let reader = std::io::BufReader::new(command.stdout.take().unwrap());
    for message in cargo_metadata::Message::parse_stream(reader) {
        match message.unwrap() {
            cargo_metadata::Message::CompilerArtifact(artifact) => artifacts.push(artifact),
            cargo_metadata::Message::BuildFinished(finished) => {
                if finished.success != true {
                    // If the build was not successful, forward the Cargo exit code.
                    std::process::exit(if let Some(code) = command.wait().unwrap().code() {
                        code
                    } else {
                        1
                    });
                }
            }
            _ => {}
        }
    }

    if let Some(artifact) = artifacts.iter().find(|a| a.target.name == name) {
        let target = &artifact.target;

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
                    std::process::exit(1);
                }
            };

            // Determine the path to the artifact.
            let filepath = filename.parent().unwrap();
            let filename = filename.file_name().unwrap();

            // HACK: Solely use forward slashes to ensure compatibility with Linux-based tooling.
            let filepath = PathBuf::from(filepath).to_slash().unwrap();

            match flavor {
                LinkerFlavor::MSVC => {
                    // These additional libraries are typically required by Rust.
                    // FIXME: Figure out when, and why?
                    const ADDITIONAL_LIBS: &str = "Bcrypt.lib Userenv.lib Ole32.lib OleAut32.lib";

                    // N.B: MSVC linker and archiver take the same command line argument format.
                    println!("/LIBPATH:{filepath} {filename} {ADDITIONAL_LIBS}");
                }
                LinkerFlavor::GCC => match dump_type {
                    DumpType::Archiver => println!("{filepath}/{name}"),
                    DumpType::Linker => println!("-L{filepath} -l{name}"),
                },
            }
        }
    } else {
        eprintln!("Could not find an artifact named \"{name}\"!");
        eprintln!("Possible artifacts:");

        // FIXME: Dependency artifacts are also listed here, but it would be improper for
        // a user to be able to directly specify the artifacts from a dependency crate
        for artifact in &artifacts {
            let name = &artifact.target.name;
            let kinds = &artifact.target.kind;

            eprintln!("  {name}: {kinds:?}");
        }

        std::process::exit(1);
    }

    let output = command.wait().unwrap();

    std::process::exit(if let Some(code) = output.code() {
        code
    } else {
        1
    });
}
