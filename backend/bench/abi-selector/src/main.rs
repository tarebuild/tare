use anyhow::Result;
fn main() -> Result<()> {
    for argument in std::env::args().skip(1) {
        let reason =
            tare_abi_selector::account_layout_observation(std::path::Path::new(&argument))?;
        println!(
            "{}",
            serde_json::json!({"source":argument,"sdkReason":reason})
        );
    }
    Ok(())
}
