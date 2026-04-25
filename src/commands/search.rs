use serde_json::{Value, json};

use crate::api::ResyClient;
use crate::cli::SearchArgs;
use crate::error::AppError;
use crate::util::to_json_value;

pub async fn run(client: &ResyClient, args: SearchArgs) -> Result<Value, AppError> {
    let raw = client
        .search(&args.query, args.limit, args.lat, args.lng)
        .await?;
    let venues: Vec<Value> = raw
        .search
        .hits
        .iter()
        .map(|hit| {
            Ok(json!({
                "id": hit.id.as_ref().and_then(|id| id.resy),
                "name": hit.name,
                "locality": hit.locality,
                "neighborhood": hit.neighborhood,
                "cuisine": hit.cuisine,
                "rating": hit.rating.as_ref().and_then(|r| r.average),
                "raw": to_json_value(hit)?,
            }))
        })
        .collect::<Result<_, AppError>>()?;

    Ok(json!({
        "ok": true,
        "query": args.query,
        "count": venues.len(),
        "venues": venues,
        "raw": raw,
    }))
}
