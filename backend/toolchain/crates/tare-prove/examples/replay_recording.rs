use anyhow::{bail, Result};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 {
        bail!("replay_recording TESTS_JSON STOCK_SO TARE_SO REPORT_JSON")
    }
    let corpus: tare_prove::RecordedCorpus = serde_json::from_slice(&std::fs::read(&args[0])?)?;
    let report = tare_prove::run_recorded(
        &std::fs::read(&args[1])?,
        &std::fs::read(&args[2])?,
        &corpus,
    )?;
    std::fs::write(&args[3], serde_json::to_vec_pretty(&report)?)?;
    println!(
        "differential: {}; validator reproduction: {}/{}",
        if report.differential.matched() {
            "matched"
        } else {
            "diverged"
        },
        report.validator_reproduction.matched,
        report.validator_reproduction.inputs
    );
    if !report.differential.matched() {
        std::process::exit(1)
    }
    if report.validator_reproduction.failed > 0 {
        std::process::exit(2)
    }
    Ok(())
}
