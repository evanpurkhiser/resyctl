use serde_json::{Value, json};

use crate::api::ResyClient;
use crate::cli::PaymentMethodsArgs;
use crate::error::AppError;
use crate::util::to_json_value;

pub async fn run(client: &ResyClient, _args: PaymentMethodsArgs) -> Result<Value, AppError> {
    let user = client.user().await?;
    let methods = user.payment_methods.unwrap_or_default();

    let entries: Vec<Value> = methods
        .iter()
        .map(|method| {
            Ok(json!({
                "id": method.id,
                "card_type": method.card_type,
                "last_4": method.last_4,
                "raw": to_json_value(method)?,
            }))
        })
        .collect::<Result<_, AppError>>()?;

    Ok(json!({
        "ok": true,
        "count": entries.len(),
        "payment_methods": entries,
    }))
}
