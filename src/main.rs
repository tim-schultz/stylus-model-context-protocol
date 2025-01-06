use std::env;
use stylus_context_provider::StylusContract;

fn main() {
    let args: Vec<String> = env::args().collect();
    let dir = args
        .get(1)
        .map(|s| s.as_str())
        .expect("You must pass a directory");

    let contract = StylusContract::new(dir);
    match contract.analyze() {
        Ok(analysis) => println!("{}", analysis),
        Err(e) => eprintln!("Error analyzing directory: {:?}", e),
    }
}
