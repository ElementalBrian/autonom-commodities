use autonom::providers::cfd::NinjasCfd;
use autonom::providers::CfdProvider;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let sym = std::env::args().nth(1).unwrap_or_else(|| "LEAN_HOGS_PERP".to_string());
    let ninjas = NinjasCfd::from_env()?;
    let q = ninjas.latest(&sym).await?;
    println!("{} -> price={} ts_ms={}", sym, q.price, q.ts_ms);
    Ok(())
}
