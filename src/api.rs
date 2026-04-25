use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::{Value, json};

use crate::error::AppError;
use crate::types::SlotId;

#[derive(Clone)]
pub struct ResyClient {
    http: reqwest::Client,
}

impl ResyClient {
    pub fn new(api_key: &str, auth_token: &str) -> Result<Self, AppError> {
        let mut headers = HeaderMap::new();
        let auth = format!("ResyAPI api_key=\"{}\"", api_key);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth)
                .map_err(|_| AppError::new(5, "invalid API key for header"))?,
        );
        headers.insert(
            "x-resy-universal-auth",
            HeaderValue::from_str(auth_token)
                .map_err(|_| AppError::new(5, "invalid auth token for header"))?,
        );
        headers.insert(
            "x-resy-auth-token",
            HeaderValue::from_str(auth_token)
                .map_err(|_| AppError::new(5, "invalid auth token for header"))?,
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent("ressy-cli/0.1.0")
            .build()
            .map_err(|e| AppError::new(4, format!("failed to build HTTP client: {e}")))?;

        Ok(Self { http })
    }

    pub fn unauthenticated(api_key: &str) -> Result<Self, AppError> {
        let mut headers = HeaderMap::new();
        let auth = format!("ResyAPI api_key=\"{}\"", api_key);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth)
                .map_err(|_| AppError::new(5, "invalid API key for header"))?,
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent("ressy-cli/0.1.0")
            .build()
            .map_err(|e| AppError::new(4, format!("failed to build HTTP client: {e}")))?;

        Ok(Self { http })
    }

    pub async fn auth_password(&self, email: &str, password: &str) -> Result<Value, AppError> {
        let response = self
            .http
            .post("https://api.resy.com/3/auth/password")
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&[("email", email), ("password", password)])
            .send()
            .await
            .map_err(|e| AppError::new(4, format!("auth request failed: {e}")))?;

        read_json_response(response).await
    }

    pub async fn user(&self) -> Result<Value, AppError> {
        let response = self
            .http
            .get("https://api.resy.com/2/user")
            .send()
            .await
            .map_err(|e| AppError::new(4, format!("user request failed: {e}")))?;
        read_json_response(response).await
    }

    pub async fn search(&self, query: &str, limit: u32, lat: f64, lng: f64) -> Result<Value, AppError> {
        let body = json!({
            "query": query,
            "per_page": limit,
            "types": ["venue"],
            "geo": { "latitude": lat, "longitude": lng }
        });
        self.post_json("https://api.resy.com/3/venuesearch/search", body)
            .await
    }

    pub async fn find(
        &self,
        venue_id: i64,
        day: &str,
        party_size: u8,
        lat: f64,
        lng: f64,
    ) -> Result<Value, AppError> {
        let response = self
            .http
            .get("https://api.resy.com/4/find")
            .query(&[
                ("venue_id", venue_id.to_string()),
                ("day", day.to_string()),
                ("party_size", party_size.to_string()),
                ("lat", lat.to_string()),
                ("long", lng.to_string()),
            ])
            .send()
            .await
            .map_err(|e| AppError::new(4, format!("find request failed: {e}")))?;

        read_json_response(response).await
    }

    pub async fn details(&self, slot: &SlotId) -> Result<Value, AppError> {
        let body = json!({
            "commit": 1,
            "config_id": slot.config_id,
            "day": slot.day,
            "party_size": slot.party_size,
        });
        self.post_json("https://api.resy.com/3/details", body).await
    }

    pub async fn book(&self, book_token: &str, payment_method_id: Option<i64>) -> Result<Value, AppError> {
        let mut form = vec![("book_token", book_token.to_string())];
        if let Some(id) = payment_method_id {
            let payment = json!({ "id": id }).to_string();
            form.push(("struct_payment_method", payment));
        }

        let response = self
            .http
            .post("https://api.resy.com/3/book")
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&form)
            .send()
            .await
            .map_err(|e| AppError::new(4, format!("book request failed: {e}")))?;

        read_json_response(response).await
    }

    async fn post_json(&self, url: &str, body: Value) -> Result<Value, AppError> {
        let response = self
            .http
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::new(4, format!("request failed: {e}")))?;

        read_json_response(response).await
    }
}

async fn read_json_response(response: reqwest::Response) -> Result<Value, AppError> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| AppError::new(4, format!("failed reading response body: {e}")))?;

    let parsed = serde_json::from_str::<Value>(&body)
        .unwrap_or_else(|_| json!({"raw": body, "parse_error": true}));

    if !status.is_success() {
        return Err(AppError::new(
            4,
            format!("api error {}: {}", status.as_u16(), parsed),
        ));
    }

    Ok(parsed)
}
