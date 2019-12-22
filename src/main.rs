#[allow(dead_code)]
mod path_util;
mod walk;

use structopt::StructOpt;

fn out_json(entry: &str) {
    println!(r#"{{"path":"{}"}}"#, entry);
}

#[derive(Debug, StructOpt)]
struct Cli {
    /// Git repo search
    #[structopt(short, long)]
    git: bool,

    /// Input file to read
    basedir: String,
}

fn main() {
    let args = Cli::from_args();
    if args.git {
        walk::find_repos(&args.basedir, out_json);
    } else {
        walk::find(&args.basedir, out_json);
    }
}
