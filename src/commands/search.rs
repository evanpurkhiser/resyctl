use serde_json::{Value, json};

use crate::api::ResyClient;
use crate::cli::SearchArgs;
use crate::error::AppError;

pub async fn run(client: &ResyClient, args: SearchArgs) -> Result<Value, AppError> {
    let raw = client.search(&args.query, args.limit, args.lat, args.lng).await?;
    let hits = raw
        .get("search")
        .and_then(|v| v.get("hits"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let venues: Vec<Value> = hits
        .iter()
        .map(|hit| {
            json!({
                "id": hit.pointer("/id/resy").and_then(Value::as_i64),
                "name": hit.get("name").and_then(Value::as_str),
                "locality": hit.get("locality").and_then(Value::as_str),
                "neighborhood": hit.get("neighborhood").and_then(Value::as_str),
                "cuisine": hit.get("cuisine").cloned(),
                "rating": hit.pointer("/rating/average").and_then(Value::as_f64),
                "raw": hit,
            })
        })
        .collect();

    Ok(json!({
        "ok": true,
        "query": args.query,
        "count": venues.len(),
        "venues": venues,
    }))
}
