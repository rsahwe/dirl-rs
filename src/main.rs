use std::{path::PathBuf, sync::LazyLock, usize};

use clap::{CommandFactory, Parser};
use glob::glob;

static USIZE_MAX_STR: LazyLock<String> = LazyLock::new(|| usize::MAX.to_string());

/// A program to mimic the `dir' windows command
#[derive(Parser, Debug)]
#[command(version, author, about, long_about = None)]
struct Args {
    #[cfg(feature = "mangen")]
    #[arg(long)]
    mangen: bool,
    /// Specify the path from which the command gets executed
    #[arg(short = 'C', long = "directory", default_value = ".")]
    path: String,
    /// Optional file pattern, may include '/'
    #[arg(default_value = "*")]
    file: String,
    /// Sets the recursive mode. 0 is the default and infinite is the default if no depth is specified.
    #[arg(short = 's', long, default_value = "0", require_equals = true, num_args = 0..=1, default_missing_value = &USIZE_MAX_STR as &String as &str)]
    depth: usize,
    /// Strips extra information from the output
    #[arg(short, long)]
    bare: bool,
    /// 'Quiet' mode that only prints extra information
    #[arg(short, long)]
    quiet: bool,
    /// Display all files and directories, even ones starting with '.'
    #[arg(short, long)]
    all: bool,
    /// No color output
    #[arg(short, long)]
    raw: bool,
}

fn main() {
    let args = Args::parse();

    #[cfg(feature = "mangen")]
    if args.mangen {
        clap_mangen::Man::new(
                <Args as clap::CommandFactory>::command()
                    .mut_arg("mangen", |arg| arg.hide(true))
            )
                .render(&mut std::io::stdout())
                .expect("Failed to render man page!");
        return;
    }

    if !PathBuf::from(args.path.clone()).is_dir() || PathBuf::from(args.path.clone()).read_dir().is_err() {
        arg_error("path".to_string(), args.path);
    }

    if args.quiet && args.bare {
        return;
    }

    let directories_only;
    let file_os;
    
    if args.file.ends_with('.') {
        directories_only = true;
        let mut new_file = args.file.clone();
        new_file.pop();
        file_os = PathBuf::from(new_file);
    } else {
        directories_only = false;
        file_os = PathBuf::from(args.file.clone());
    }

    let path_os = PathBuf::from(args.path.clone());

    let stats = dir_cmd_recursive(&args, path_os, &file_os, directories_only, args.depth);
    if !args.bare {
        print_end_stats(stats.0, stats.1, stats.2);
    }
}

fn arg_error(arg: String, value: String) -> ! {
    let mut err = clap::Error::new(clap::error::ErrorKind::ValueValidation).with_cmd(&Args::command());
    err.insert(clap::error::ContextKind::InvalidArg, clap::error::ContextValue::String(arg));
    err.insert(clap::error::ContextKind::InvalidValue, clap::error::ContextValue::String(value));
    err.exit();
}

fn dir_cmd_recursive(args: &Args, current_path: PathBuf, file_pattern: &PathBuf, directories_only: bool, depth: usize) -> (usize, usize, usize) {
    let mut files = 0;
    let mut file_size_sum = 0;
    let mut directories = 0;

    let glob = match glob(current_path.clone().join(file_pattern.clone()).to_str().unwrap()) {
        Ok(paths) => paths,
        Err(_) =>  return (0, 0, 0),
    };

    for path in glob.filter_map(|p| match p { Ok(p) => Some(p), Err(_) => None }) {
        let name = match path.file_name() {
            Some(name) => name,
            None => continue,
        };
        if !args.all && name.as_encoded_bytes()[0] == b'.' {
            continue;
        }


        if path.is_file() && !directories_only {
            files += 1;
            let file_size = path.metadata().unwrap().len() as usize;
            file_size_sum += file_size;
            if !args.quiet {
                if args.raw {
                    println!("<FILE>\t{}\t{} bytes", path.canonicalize().unwrap().display(), file_size);
                } else {
                    println!("\x1b[1;32m<FILE>\x1b[0m\t{}\t{} bytes", path.canonicalize().unwrap().display(), file_size);
                }
            }
        } else if path.is_dir() {
            directories += 1;
            if !args.quiet {
                if args.raw {
                    println!("<DIR>\t{}", path.canonicalize().unwrap().display());
                } else {
                    println!("\x1b[1;35m<DIR>\x1b[0m\t{}", path.canonicalize().unwrap().display());
                }
            }
        }
    }

    if depth != 0 {
        if let Ok(read_dir) = current_path.read_dir() {
            for path in read_dir.filter_map(Result::ok).map(|ent| ent.path()).filter(|path| path.is_dir()).filter(|path| args.all || path.file_name().unwrap().as_encoded_bytes()[0] != b'.').filter(|path| !path.is_symlink()) {
                let res = dir_cmd_recursive(args, path, file_pattern, directories_only, depth - 1);
                files += res.0;
                file_size_sum += res.1;
                directories += res.2;
            }
        }
    }

    (files, file_size_sum, directories)
}

fn print_end_stats(files: usize, file_size_sum: usize, directories: usize) {
    print!("\t\t{} File(s)\t{} bytes\n\t\t{} Dir(s)\n", files, file_size_sum, directories);
}
