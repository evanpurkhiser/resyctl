use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::error::AppError;
use crate::models::{
    AuthPasswordResponse, BookResponse, CancelResponse, DetailsResponse, FindResponse,
    ReservationLookupResponse, SearchResponse, UserResponse,
};

#[derive(Clone)]
pub struct ResyClient {
    http: reqwest::Client,
    base_url: String,
}

impl ResyClient {
    pub fn new(api_key: &str, auth_token: &str) -> Result<Self, AppError> {
        Self::new_with_base_url(api_key, auth_token, "https://api.resy.com")
    }

    pub fn new_with_base_url(
        api_key: &str,
        auth_token: &str,
        base_url: &str,
    ) -> Result<Self, AppError> {
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
            .user_agent("resyctl/0.1.0")
            .build()
            .map_err(|e| AppError::new(4, format!("failed to build HTTP client: {e}")))?;

        Ok(Self {
            http,
            base_url: normalize_base_url(base_url),
        })
    }

    pub fn unauthenticated(api_key: &str) -> Result<Self, AppError> {
        Self::unauthenticated_with_base_url(api_key, "https://api.resy.com")
    }

    pub fn unauthenticated_with_base_url(api_key: &str, base_url: &str) -> Result<Self, AppError> {
        let mut headers = HeaderMap::new();
        let auth = format!("ResyAPI api_key=\"{}\"", api_key);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth)
                .map_err(|_| AppError::new(5, "invalid API key for header"))?,
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent("resyctl/0.1.0")
            .build()
            .map_err(|e| AppError::new(4, format!("failed to build HTTP client: {e}")))?;

        Ok(Self {
            http,
            base_url: normalize_base_url(base_url),
        })
    }

    pub async fn auth_password(
        &self,
        email: &str,
        password: &str,
    ) -> Result<AuthPasswordResponse, AppError> {
        let response = self
            .http
            .post(self.endpoint("/3/auth/password"))
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&[("email", email), ("password", password)])
            .send()
            .await
            .map_err(|e| AppError::new(4, format!("auth request failed: {e}")))?;

        read_json_response(response).await
    }

    pub async fn user(&self) -> Result<UserResponse, AppError> {
        let response = self
            .http
            .get(self.endpoint("/2/user"))
            .send()
            .await
            .map_err(|e| AppError::new(4, format!("user request failed: {e}")))?;
        read_json_response(response).await
    }

    pub async fn search(
        &self,
        query: &str,
        limit: u32,
        lat: f64,
        lng: f64,
    ) -> Result<SearchResponse, AppError> {
        let body = json!({
            "query": query,
            "per_page": limit,
            "types": ["venue"],
            "geo": { "latitude": lat, "longitude": lng }
        });
        self.post_json_typed(&self.endpoint("/3/venuesearch/search"), body)
            .await
    }

    pub async fn find(
        &self,
        venue_id: i64,
        day: &str,
        party_size: u8,
        lat: f64,
        lng: f64,
    ) -> Result<FindResponse, AppError> {
        let response = self
            .http
            .get(self.endpoint("/4/find"))
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

    pub async fn details_with_commit(
        &self,
        config_id: &str,
        commit: i32,
    ) -> Result<DetailsResponse, AppError> {
        let body = json!({
            "config_id": config_id,
            "commit": commit,
            "struct_items": [],
        });
        self.post_json_typed(&self.endpoint("/3/details"), body)
            .await
    }

    pub async fn reservations(
        &self,
        resy_token: Option<&str>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<ReservationLookupResponse, AppError> {
        let mut query: Vec<(&str, String)> = Vec::new();
        if let Some(token) = resy_token {
            query.push(("resy_token", token.to_string()));
        }
        if let Some(limit) = limit {
            query.push(("limit", limit.to_string()));
        }
        if let Some(offset) = offset {
            query.push(("offset", offset.to_string()));
        }

        let response = self
            .http
            .get(self.endpoint("/3/user/reservations"))
            .query(&query)
            .send()
            .await
            .map_err(|e| AppError::new(4, format!("reservations request failed: {e}")))?;

        read_json_response(response).await
    }

    pub async fn reservation_by_token(
        &self,
        resy_token: &str,
    ) -> Result<ReservationLookupResponse, AppError> {
        self.reservations(Some(resy_token), None, None).await
    }

    pub async fn cancel(&self, resy_token: &str) -> Result<CancelResponse, AppError> {
        let response = self
            .http
            .post(self.endpoint("/3/cancel"))
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&[("resy_token", resy_token)])
            .send()
            .await
            .map_err(|e| AppError::new(4, format!("cancel request failed: {e}")))?;

        read_json_response(response).await
    }

    pub async fn book(
        &self,
        book_token: &str,
        payment_method_id: Option<i64>,
        replace: bool,
        venue_marketing_opt_in: bool,
    ) -> Result<BookResponse, AppError> {
        let mut form = vec![("book_token", book_token.to_string())];
        if let Some(id) = payment_method_id {
            let payment = json!({ "id": id }).to_string();
            form.push(("struct_payment_method", payment));
        }
        form.push(("replace", if replace { "1" } else { "0" }.to_string()));
        form.push((
            "venue_marketing_opt_in",
            if venue_marketing_opt_in { "1" } else { "0" }.to_string(),
        ));

        let response = self
            .http
            .post(self.endpoint("/3/book"))
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&form)
            .send()
            .await
            .map_err(|e| AppError::new(4, format!("book request failed: {e}")))?;

        read_json_response(response).await
    }

    async fn post_json_typed<T: DeserializeOwned>(
        &self,
        url: &str,
        body: Value,
    ) -> Result<T, AppError> {
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

    fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

fn normalize_base_url(base_url: &str) -> String {
    base_url.trim_end_matches('/').to_string()
}

async fn read_json_response<T: DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, AppError> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| AppError::new(4, format!("failed reading response body: {e}")))?;

    let parsed = serde_json::from_str::<Value>(&body).map_err(|e| {
        AppError::new(
            4,
            format!("failed to deserialize API response body as JSON: {e}"),
        )
    })?;

    if !status.is_success() {
        return Err(AppError::new(
            4,
            format!("api error {}: {}", status.as_u16(), parsed),
        ));
    }

    serde_json::from_value::<T>(parsed)
        .map_err(|e| AppError::new(4, format!("failed to deserialize API response: {e}")))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use httpmock::Method::{GET, POST};
    use httpmock::MockServer;

    use super::*;

    fn fixture(name: &str) -> String {
        let path = format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name);
        fs::read_to_string(path).expect("fixture should exist")
    }

    fn authed_client(server: &MockServer) -> ResyClient {
        ResyClient::new_with_base_url("test-api-key", "test-auth-token", &server.base_url())
            .expect("client should build")
    }

    #[tokio::test]
    async fn search_deserializes_fixture_response() {
        let server = MockServer::start();
        let body = fixture("search_response.json");

        let mock = server.mock(|when, then| {
            when.method(POST).path("/3/venuesearch/search");
            then.status(200)
                .header("content-type", "application/json")
                .body(body);
        });

        let client = authed_client(&server);
        let response = client.search("ishq", 5, 40.7, -73.9).await.unwrap();

        mock.assert();
        assert_eq!(response.search.hits.len(), 2);
        assert_eq!(
            response.search.hits[0].id.as_ref().and_then(|id| id.resy),
            Some(84214)
        );
    }

    #[tokio::test]
    async fn find_deserializes_fixture_response() {
        let server = MockServer::start();
        let body = fixture("find_response.json");

        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/4/find")
                .query_param("venue_id", "84214")
                .query_param("day", "2026-04-26")
                .query_param("party_size", "2");
            then.status(200)
                .header("content-type", "application/json")
                .body(body);
        });

        let client = authed_client(&server);
        let response = client
            .find(84214, "2026-04-26", 2, 40.7128, -74.006)
            .await
            .unwrap();

        mock.assert();
        let first_slot = &response.results.unwrap().venues[0].slots[0];
        assert_eq!(
            first_slot.config.as_ref().and_then(|c| c.kind.as_deref()),
            Some("Bar Seat")
        );
        assert_eq!(
            first_slot.payment.as_ref().and_then(|p| p.cancellation_fee),
            Some(25.0)
        );
    }

    #[tokio::test]
    async fn details_deserializes_quote_fixture() {
        let server = MockServer::start();
        let body = fixture("details_commit0_response.json");

        let mock = server.mock(|when, then| {
            when.method(POST).path("/3/details");
            then.status(200)
                .header("content-type", "application/json")
                .body(body);
        });

        let client = authed_client(&server);
        let response = client
            .details_with_commit("rgs://resy/config-token", 0)
            .await
            .unwrap();

        mock.assert();
        assert_eq!(
            response
                .cancellation
                .as_ref()
                .and_then(|c| c.fee.as_ref())
                .and_then(|f| f.amount),
            Some(25.0)
        );
        assert!(response.book_token.is_none());
    }

    #[tokio::test]
    async fn details_deserializes_commit_fixture_with_token() {
        let server = MockServer::start();
        let body = fixture("details_commit1_response.json");

        let mock = server.mock(|when, then| {
            when.method(POST).path("/3/details");
            then.status(201)
                .header("content-type", "application/json")
                .body(body);
        });

        let client = authed_client(&server);
        let response = client
            .details_with_commit("rgs://resy/config-token", 1)
            .await
            .unwrap();

        mock.assert();
        assert_eq!(
            response.book_token.and_then(|t| t.value),
            Some("book-token-xyz".to_string())
        );
    }

    #[tokio::test]
    async fn book_deserializes_success_fixture() {
        let server = MockServer::start();
        let body = fixture("book_response.json");

        let mock = server.mock(|when, then| {
            when.method(POST).path("/3/book");
            then.status(201)
                .header("content-type", "application/json")
                .body(body);
        });

        let client = authed_client(&server);
        let response = client
            .book("book-token-xyz", Some(31340008), false, false)
            .await
            .unwrap();

        mock.assert();
        assert_eq!(response.reservation_id, Some(867413540));
        assert_eq!(response.resy_token, Some("resy-token-abc".to_string()));
    }

    #[tokio::test]
    async fn reservations_deserializes_fixture_response() {
        let server = MockServer::start();
        let body = fixture("reservations_response.json");

        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/3/user/reservations")
                .query_param("limit", "10")
                .query_param("offset", "0");
            then.status(200)
                .header("content-type", "application/json")
                .body(body);
        });

        let client = authed_client(&server);
        let response = client.reservations(None, Some(10), Some(0)).await.unwrap();

        mock.assert();
        assert_eq!(response.reservations.len(), 2);
        assert_eq!(response.reservations[0].reservation_id, Some(867250480));
    }

    #[tokio::test]
    async fn cancel_deserializes_fixture_response() {
        let server = MockServer::start();
        let body = fixture("cancel_response.json");

        let mock = server.mock(|when, then| {
            when.method(POST).path("/3/cancel");
            then.status(200)
                .header("content-type", "application/json")
                .body(body);
        });

        let client = authed_client(&server);
        let response = client.cancel("resy-token-abc").await.unwrap();

        mock.assert();
        assert_eq!(
            response
                .payment
                .and_then(|p| p.transaction)
                .and_then(|t| t.refund),
            Some(1)
        );
    }

    #[tokio::test]
    async fn typed_methods_return_app_error_on_non_success() {
        let server = MockServer::start();
        let body = fixture("not_found_error.json");

        let mock = server.mock(|when, then| {
            when.method(GET).path("/3/user/reservations");
            then.status(404)
                .header("content-type", "application/json")
                .body(body);
        });

        let client = authed_client(&server);
        let err = client.reservations(None, None, None).await.unwrap_err();

        mock.assert();
        assert_eq!(err.code, 4);
        assert!(err.message.contains("api error 404"));
    }
}
