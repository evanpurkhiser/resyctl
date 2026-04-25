use serde_json::{Value, json};

use crate::api::ResyClient;
use crate::cli::PaymentMethodsArgs;
use crate::error::Error;
use crate::util::to_json_value;

pub async fn run(client: &ResyClient, _args: PaymentMethodsArgs) -> Result<Value, Error> {
    let user = client.user().await?;
    let methods = user.payment_methods.clone().unwrap_or_default();

    let entries: Vec<Value> = methods
        .iter()
        .map(|method| {
            json!({
                "id": method.id,
                "card_type": method.card_type,
                "last_4": method.last_4,
            })
        })
        .collect();

    Ok(json!({
        "ok": true,
        "count": entries.len(),
        "payment_methods": entries,
        "raw": to_json_value(&user)?,
    }))
}
