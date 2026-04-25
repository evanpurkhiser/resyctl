use serde_json::{Value, json};

use crate::api::ResyClient;
use crate::cli::SearchArgs;
use crate::error::Error;

pub async fn run(client: &ResyClient, args: SearchArgs) -> Result<Value, Error> {
    let raw = client
        .search(&args.query, args.limit, args.lat, args.lng)
        .await?;
    let venues: Vec<Value> = raw
        .search
        .hits
        .iter()
        .map(|hit| {
            json!({
                "id": hit.id.as_ref().and_then(|id| id.resy),
                "name": hit.name,
                "locality": hit.locality,
                "neighborhood": hit.neighborhood,
                "cuisine": hit.cuisine,
                "rating": hit.rating.as_ref().and_then(|r| r.average),
            })
        })
        .collect();

    Ok(json!({
        "ok": true,
        "query": args.query,
        "count": venues.len(),
        "venues": venues,
        "raw": raw,
    }))
}
