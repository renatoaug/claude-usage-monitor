// What a turn would have cost on the API: the list prices per model, in USD
// per million tokens (Anthropic's for Claude, OpenAI's for Codex). A
// subscription pays none of this; it is the value of the tokens, not a bill.
// `prices.json` in the data dir adds to the list or corrects it.

use serde_json::Value;

/// One model's list prices, USD per million tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rates {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
}

// a cache write costs the input price times this, by how long it is kept
const WRITE_5M: f64 = 1.25;
const WRITE_1H: f64 = 2.0;

// First match wins, so a newer id comes before the one it starts with
// (`opus-5-5` before `opus-5`). Legacy ids put the version first
// (`claude-3-5-haiku`), and fall to their family's last row.
const TABLE: [(&str, Rates); 21] = [
    ("fable-5-2", Rates { input: 8.0, output: 40.0, cache_read: 0.20 }),
    ("mythos-5-2", Rates { input: 8.0, output: 40.0, cache_read: 0.20 }),
    ("fable-5-1", Rates { input: 10.0, output: 50.0, cache_read: 0.25 }),
    ("mythos-5-1", Rates { input: 10.0, output: 50.0, cache_read: 0.25 }),
    ("fable", Rates { input: 10.0, output: 50.0, cache_read: 1.0 }),
    ("mythos", Rates { input: 10.0, output: 50.0, cache_read: 1.0 }),
    ("opus-5-5", Rates { input: 4.0, output: 20.0, cache_read: 0.20 }),
    ("opus-5", Rates { input: 5.0, output: 25.0, cache_read: 0.50 }),
    ("opus-4-8", Rates { input: 5.0, output: 25.0, cache_read: 0.50 }),
    ("opus-4-7", Rates { input: 5.0, output: 25.0, cache_read: 0.50 }),
    ("opus-4-6", Rates { input: 5.0, output: 25.0, cache_read: 0.50 }),
    ("opus-4-5", Rates { input: 5.0, output: 25.0, cache_read: 0.50 }),
    ("opus", Rates { input: 15.0, output: 75.0, cache_read: 1.50 }),
    ("sonnet-5-5", Rates { input: 2.0, output: 10.0, cache_read: 0.10 }),
    ("sonnet-5", Rates { input: 2.0, output: 10.0, cache_read: 0.20 }),
    ("sonnet", Rates { input: 3.0, output: 15.0, cache_read: 0.30 }),
    ("haiku-5-5", Rates { input: 0.10, output: 0.50, cache_read: 0.01 }),
    ("haiku-4-5", Rates { input: 1.0, output: 5.0, cache_read: 0.10 }),
    ("3-5-haiku", Rates { input: 0.80, output: 4.0, cache_read: 0.08 }),
    ("haiku-3-5", Rates { input: 0.80, output: 4.0, cache_read: 0.08 }),
    ("haiku", Rates { input: 0.25, output: 1.25, cache_read: 0.03 }),
];

const fn openai(input: f64, cache_read: f64, output: f64) -> Rates {
    Rates { input, output, cache_read }
}

// OpenAI's standard-tier prices (short context), as its pricing page listed
// them on 2026-11-24. Matched on the exact id (or the id and a date): a
// `gpt-5.1-codex` is not priced as a `gpt-5.1`. The pro models have no
// cached rate, so a cached token costs them the input price.
const OPENAI: [(&str, Rates); 24] = [
    ("gpt-6.2-astra", openai(8.0, 0.40, 40.0)),
    ("gpt-6.2-sol", openai(2.0, 0.10, 10.0)),
    ("gpt-6.1-sol", openai(2.0, 0.10, 10.0)),
    ("gpt-6.1-terra", openai(1.0, 0.05, 5.0)),
    ("gpt-6-astra", openai(10.0, 1.0, 50.0)),
    ("gpt-6-sol", openai(2.0, 0.20, 10.0)),
    ("gpt-6-luna", openai(0.10, 0.01, 0.50)),
    ("gpt-5.6-sol", openai(4.0, 0.40, 20.0)),
    ("gpt-5.6-terra", openai(2.0, 0.20, 12.0)),
    ("gpt-5.6-luna", openai(0.20, 0.02, 1.20)),
    ("gpt-5.5", openai(5.0, 0.50, 30.0)),
    ("gpt-5.5-pro", openai(30.0, 30.0, 180.0)),
    ("gpt-5.4", openai(2.50, 0.25, 15.0)),
    ("gpt-5.4-pro", openai(30.0, 30.0, 180.0)),
    ("gpt-5.4-mini", openai(0.75, 0.075, 4.50)),
    ("gpt-5.4-nano", openai(0.20, 0.02, 1.25)),
    ("gpt-5.3-codex", openai(1.75, 0.175, 14.0)),
    ("gpt-5.2", openai(1.75, 0.175, 14.0)),
    ("gpt-5.2-pro", openai(21.0, 21.0, 168.0)),
    ("gpt-5.1", openai(1.25, 0.125, 10.0)),
    ("gpt-5", openai(1.25, 0.125, 10.0)),
    ("gpt-5-mini", openai(0.25, 0.025, 2.0)),
    ("gpt-5-nano", openai(0.05, 0.005, 0.40)),
    ("gpt-5-pro", openai(15.0, 15.0, 120.0)),
];

/// `prices.json` as a list: `{"<model id or part of one>": {"input": 4,
/// "output": 20, "cacheRead": 0.2}}`, USD per million tokens. A cache read
/// left out costs a tenth of the input. The longest name comes first, so the
/// most specific one wins.
pub fn parse_overrides(v: &Value) -> Vec<(String, Rates)> {
    let Some(map) = v.as_object() else { return vec![] };
    let mut out: Vec<(String, Rates)> = map
        .iter()
        .filter_map(|(name, r)| {
            let price = |k: &str| r.get(k).and_then(Value::as_f64).filter(|p| *p >= 0.0);
            let input = price("input")?;
            Some((name.to_lowercase(), Rates { input, output: price("output")?, cache_read: price("cacheRead").unwrap_or(input * 0.1) }))
        })
        .filter(|(name, _)| !name.is_empty())
        .collect();
    out.sort_by_key(|(name, _)| std::cmp::Reverse(name.len()));
    out
}

// read once, at the first price asked: an edit takes a restart
static OVERRIDES: std::sync::LazyLock<Vec<(String, Rates)>> = std::sync::LazyLock::new(|| {
    // the tests price from the built-in list alone, whatever this machine holds
    if cfg!(test) {
        return vec![];
    }
    let path = crate::auth::default_data_dir().join("prices.json");
    std::fs::read(path).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).map(|v| parse_overrides(&v)).unwrap_or_default()
});

/// The list prices for a model id: `prices.json` first, then the built-in
/// lists. None for a model nobody listed: it is left out of the total rather
/// than priced by a guess.
pub fn rates(model: &str) -> Option<Rates> {
    rates_with(&OVERRIDES, model)
}

fn rates_with(overrides: &[(String, Rates)], model: &str) -> Option<Rates> {
    let m = model.to_lowercase();
    if let Some((_, r)) = overrides.iter().find(|(name, _)| m.contains(name)) {
        return Some(*r);
    }
    let dated = |key: &str| m.strip_prefix(key).is_some_and(|rest| rest.is_empty() || rest.starts_with("-20"));
    OPENAI.iter().find(|(key, _)| dated(key)).or_else(|| TABLE.iter().find(|(key, _)| m.contains(key))).map(|(_, r)| *r)
}

/// What a Codex answer cost: OpenAI counts the cached tokens inside the
/// input, and the reasoning inside the output.
pub fn openai_cost(model: Option<&str>, input: u64, cached: u64, output: u64) -> f64 {
    let Some(r) = model.and_then(rates) else { return 0.0 };
    let cached = cached.min(input);
    ((input - cached) as f64 * r.input + cached as f64 * r.cache_read + output as f64 * r.output) / 1e6
}

/// What the plan costs a month, in USD: the price set in Settings, else the
/// one plan with a single price. Max comes at several, so it is never guessed.
pub fn plan_price(plan: Option<&str>, set: i32) -> Option<f64> {
    match (set, plan) {
        (p, _) if p > 0 => Some(p as f64),
        (_, Some(plan)) if plan.eq_ignore_ascii_case("pro") => Some(20.0),
        _ => None,
    }
}

/// The tokens of one answer, by how the API bills them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tokens {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    /// cache writes kept 5 minutes (the default) and kept an hour
    pub write_5m: u64,
    pub write_1h: u64,
}

/// What these tokens cost on the API, in USD; 0 for an unknown model.
pub fn cost(model: Option<&str>, t: &Tokens) -> f64 {
    let Some(r) = model.and_then(rates) else { return 0.0 };
    let input = t.input as f64 + t.write_5m as f64 * WRITE_5M + t.write_1h as f64 * WRITE_1H;
    (input * r.input + t.output as f64 * r.output + t.cache_read as f64 * r.cache_read) / 1e6
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_newer_model_is_not_priced_as_the_one_its_id_starts_with() {
        assert_eq!(rates("claude-opus-5-5").map(|r| r.input), Some(4.0));
        assert_eq!(rates("claude-opus-5").map(|r| r.input), Some(5.0));
        assert_eq!(rates("claude-opus-4-1-20250805").map(|r| r.input), Some(15.0));
        assert_eq!(rates("claude-opus-4-5-20251101").map(|r| r.input), Some(5.0));
        assert_eq!(rates("claude-sonnet-5-5").map(|r| r.cache_read), Some(0.10));
        assert_eq!(rates("claude-sonnet-4-5-20250929").map(|r| r.input), Some(3.0));
        assert_eq!(rates("claude-fable-5-2").map(|r| r.output), Some(40.0));
        assert_eq!(rates("claude-haiku-4-5-20251001").map(|r| r.input), Some(1.0));
        assert_eq!(rates("claude-3-5-haiku-20241022").map(|r| r.input), Some(0.80));
        assert_eq!(rates("some-other-model"), None);
    }

    #[test]
    fn an_openai_model_is_priced_only_under_its_own_name() {
        assert_eq!(rates("gpt-5").map(|r| r.input), Some(1.25));
        assert_eq!(rates("gpt-5.4-mini").map(|r| r.output), Some(4.50));
        assert_eq!(rates("gpt-5-2025-08-07").map(|r| r.input), Some(1.25));
        // not on the price page: no guess from the name it starts with
        assert_eq!(rates("gpt-5.1-codex"), None);
        // cached tokens are part of the input: 1M in, half of it cached, 1M out
        let m = 1_000_000;
        assert_eq!(openai_cost(Some("gpt-5"), m, m / 2, m), 0.625 + 0.0625 + 10.0);
        assert_eq!(openai_cost(Some("gpt-5.1-codex"), m, 0, m), 0.0);
    }

    #[test]
    fn prices_json_adds_to_the_list_and_corrects_it() {
        let over = parse_overrides(&serde_json::json!({
            "claude-opus": {"input": 1, "output": 2},
            "claude-opus-5-5": {"input": 3, "output": 6, "cacheRead": 0.5},
            "gpt-5.1-codex": {"input": 1.25, "output": 10, "cacheRead": 0.125},
            "broken": {"input": "free"},
        }));
        assert_eq!(over.len(), 3);
        // the most specific name wins, and a missing cache read is a tenth of the input
        assert_eq!(rates_with(&over, "claude-opus-5-5"), Some(Rates { input: 3.0, output: 6.0, cache_read: 0.5 }));
        assert_eq!(rates_with(&over, "claude-opus-5"), Some(Rates { input: 1.0, output: 2.0, cache_read: 0.1 }));
        assert_eq!(rates_with(&over, "gpt-5.1-codex").map(|r| r.input), Some(1.25));
        // what it doesn't name keeps its built-in price
        assert_eq!(rates_with(&over, "claude-sonnet-5-5").map(|r| r.input), Some(2.0));
    }

    #[test]
    fn the_plan_price_is_set_or_known_never_guessed() {
        assert_eq!(plan_price(Some("Max"), 200), Some(200.0));
        assert_eq!(plan_price(Some("Pro"), 0), Some(20.0));
        assert_eq!(plan_price(Some("Max"), 0), None);
        assert_eq!(plan_price(None, 0), None);
    }

    #[test]
    fn each_kind_of_token_is_billed_at_its_own_price() {
        let m = Some("claude-opus-5-5");
        let million = 1_000_000;
        assert_eq!(cost(m, &Tokens { input: million, ..Default::default() }), 4.0);
        assert_eq!(cost(m, &Tokens { output: million, ..Default::default() }), 20.0);
        assert_eq!(cost(m, &Tokens { cache_read: million, ..Default::default() }), 0.20);
        assert_eq!(cost(m, &Tokens { write_5m: million, ..Default::default() }), 5.0);
        assert_eq!(cost(m, &Tokens { write_1h: million, ..Default::default() }), 8.0);
        assert_eq!(cost(Some("some-other-model"), &Tokens { input: million, ..Default::default() }), 0.0);
        assert_eq!(cost(None, &Tokens { input: million, ..Default::default() }), 0.0);
    }
}
