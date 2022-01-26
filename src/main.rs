use clap::{App, Arg};
use std::process::{Command, Stdio};

fn main() {
    let matches = App::new("cargo pkgconfig")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Justin Moore <jusmoore@microsoft.com>")
        .about("Extract crate metadata with an interface similar to pkg-config")
        .args(&[
            Arg::new("name")
                .required(true)
                .takes_value(true)
                .help("Name of the library (usually the same as the crate name)"),
            Arg::new("libs")
                .help("output all linker flags")
                .long("libs")
                .required(false),
            Arg::new("cargocmd").multiple_values(true),
        ])
        .get_matches();

    let name = matches.value_of("name").unwrap();
    let dump_libs = matches.is_present("libs");

    let cargo_args = matches
        .values_of("cargocmd")
        .unwrap_or(clap::Values::default())
        .collect::<Vec<_>>();

    let cargo_args = ["build", "--message-format=json-render-diagnostics"]
        .iter()
        .chain(cargo_args.iter())
        .collect::<Vec<_>>();

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
                    if target.kind.iter().any(|s| s == "lib") && dump_libs {
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
                        println!("-L{filepath} -l{name}");
                    }
                }
            }
            cargo_metadata::Message::BuildFinished(_) => {}
            _ => {}
        }
    }

    let output = command.wait().unwrap();

    std::process::exit(if let Some(code) = output.code() {
        code
    } else {
        1
    });
}
