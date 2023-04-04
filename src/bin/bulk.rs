use clap::Parser;
use simple_stresscheck::{read_bulk, Error, Stress};
use std::fs::File;
use std::io::BufReader;

#[derive(Parser)]
struct Args {
    path: String,
}

fn main() -> Result<(), Error> {
    let args = Args::parse();
    let reader = BufReader::new(File::open(&args.path)?);
    for row in read_bulk(reader) {
        match row {
            Ok((id, store)) => match store.to_sumup_score() {
                Ok(score) => {
                    println!(
                        "id = {}, scores = {:?}, has_stress = {}",
                        id,
                        score.scores(),
                        score.has_stress()
                    );
                }
                Err(e) => {
                    dbg!("{}", e);
                }
            },
            Err(e) => {
                dbg!("{}", e);
            }
        }
    }
    Ok(())
}
