use anyhow::{Context, Result};
use mcb_core::dispatcher::dispatch;
use mcb_core::protocol::{CoreRequest, CoreResponse};
use std::io::{self, Read};

fn main() {
    if let Err(error) = run() {
        let response =
            CoreResponse::error("startup", "mcb-core failed", vec![format!("{error:#}")]);
        println!("{}", serde_json::to_string(&response).unwrap());
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .context("failed to read request from stdin")?;
    let request: CoreRequest = serde_json::from_str(&input).context("invalid core request JSON")?;
    let response = dispatch(request);
    println!("{}", serde_json::to_string(&response)?);
    Ok(())
}
