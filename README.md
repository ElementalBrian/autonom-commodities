# Autonom Operator / Oracle

Rust-based Web2/Oracle server with CFD-only consensus mode.

## Quick start
- Copy `config/oracle.example.toml` to `config/oracle.toml` and fill real values.
- Run with your usual cargo command(s).

















Here’s a quick, practical ranking of non-CME exchanges by how easy it is to get futures prices (API/docs + licensing friction):

Very easy (developer-friendly right away)

SGX — Offers a public Delayed Level-1 REST API for securities and derivatives (plug-and-play for prototyping; real-time needs a license).
Singapore Exchange

Nasdaq Commodities (Nordic power) — 15-min delayed prices on web + Nasdaq Web API Service for commodities/derivatives; commercial use requires agreement.
Nasdaq
+1

Easy via a vendor

ICE (e.g., softs, energy on IFUS) — Direct ICE data is contractual, but you can get programmatic access through vendors like DataBento (IFUS real-time dataset).
Databento
Ice

Moderate (clear path, some paperwork)

EEX (power, gas, EUAs) — Has documented Cloud Stream API (JSON/Protobuf); requires licensing for futures/real-time beyond specific spot feeds.
EEX
+1

DGCX (gold, FX, oil, single-stock futures) — Publishes a real-time pricing feed offering; apply for a market-data license.
dgcx.ae
+1

Dubai Mercantile Exchange / Gulf Mercantile (Oman crude) — Explicit derived-data licensing with a simple scope-of-use form (good for daily anchors).
gulfmerc.com

Hard (heavier licensing/cost)

LME (base metals) — Access via LMEsource with strict licensing (incl. “trading & clearing” and distribution categories). Great quality, not “easy.”
Lme
+1

INE / SHFE (China) — Formal vendor or non-display licenses required; generally higher friction for foreign firms.
ine.com.cn
shfe.com.cn

TL;DR for a perp price stack without CME: start with SGX delayed API or Nasdaq Web API to prototype, add ICE via DataBento for live benchmarks where needed, and consider EEX/DGCX once you’re ready to sign data licenses.
Singapore Exchange
Nasdaq
Databento
EEX
dgcx.ae