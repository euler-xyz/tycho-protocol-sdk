use anyhow::{Ok, Result};
use substreams_ethereum::Abigen;

fn main() -> Result<(), anyhow::Error> {
    Abigen::new("EulerSwapFactory", "abi/eulerswap_factory.json")?
        .generate()?
        .write_to_file("src/abi/eulerswap_factory.rs")?;
    Abigen::new("EulerSwap", "abi/eulerswap.json")?
        .generate()?
        .write_to_file("src/abi/eulerswap.rs")?;
    Abigen::new("EulerSwapPeriphery", "abi/eulerswap_periphery.json")?
    .generate()?
    .write_to_file("src/abi/eulerswap_periphery.rs")?;
    Ok(())
}
