use std::sync::Arc;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::{RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::error::{ApiError, Error};
use crate::models::{
    AuthPasswordResponse, BookResponse, CancelResponse, DetailsResponse, FindResponse,
    ReservationLookupResponse, SearchResponse, UserResponse,
};
use crate::state;
use crate::types::{BookToken, ConfigId, ResyToken};

#[derive(Clone, Debug)]
struct Credentials {
    email: String,
    password: String,
}

#[derive(Clone)]
pub struct ResyClient {
    http: reqwest::Client,
    base_url: String,
    client_key: String,
    auth_token: Arc<Mutex<Option<String>>>,
    credentials: Option<Credentials>,
}

impl ResyClient {
    #[cfg(test)]
    pub fn new_with_base_url(
        client_key: &str,
        auth_token: &str,
        base_url: &str,
    ) -> Result<Self, Error> {
        Self::build(client_key, base_url, Some(auth_token), None)
    }

    pub fn unauthenticated(client_key: &str) -> Result<Self, Error> {
        Self::build(client_key, "https://api.resy.com", None, None)
    }

    /// Build a client wired to persisted state: uses the saved auth token and
    /// automatically re-authenticates on a 401 if email/password are stored.
    pub fn from_state(client_key: &str) -> Result<Self, Error> {
        let s = state::load()?;
        let credentials = match (s.email.as_deref(), s.password.as_deref()) {
            (Some(email), Some(password)) if !email.is_empty() && !password.is_empty() => {
                Some(Credentials {
                    email: email.to_string(),
                    password: password.to_string(),
                })
            }
            _ => None,
        };

        if s.auth_token.is_none() && credentials.is_none() {
            return Err(Error::MissingAuthToken);
        }

        Self::build(
            client_key,
            "https://api.resy.com",
            s.auth_token.as_deref(),
            credentials,
        )
    }

    fn build(
        client_key: &str,
        base_url: &str,
        auth_token: Option<&str>,
        credentials: Option<Credentials>,
    ) -> Result<Self, Error> {
        let http = reqwest::Client::builder()
            .user_agent("resyctl/0.1.0")
            .build()
            .map_err(|e| Error::Api(ApiError::BuildClient(e)))?;

        Ok(Self {
            http,
            base_url: normalize_base_url(base_url),
            client_key: client_key.to_string(),
            auth_token: Arc::new(Mutex::new(auth_token.map(|t| t.to_string()))),
            credentials,
        })
    }

    pub async fn auth_password(
        &self,
        email: &str,
        password: &str,
    ) -> Result<AuthPasswordResponse, Error> {
        let response = self
            .http
            .post(self.endpoint("/3/auth/password"))
            .header(AUTHORIZATION, self.client_auth_header())
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&[("email", email), ("password", password)])
            .send()
            .await
            .map_err(|e| Error::Api(ApiError::AuthRequest(e)))?;

        read_json_response(response).await
    }

    pub async fn user(&self) -> Result<UserResponse, Error> {
        let url = self.endpoint("/2/user");
        let response = self.execute(|c| c.get(&url)).await?;
        read_json_response(response).await
    }

    pub async fn search(
        &self,
        query: &str,
        limit: u32,
        lat: f64,
        lng: f64,
    ) -> Result<SearchResponse, Error> {
        let url = self.endpoint("/3/venuesearch/search");
        let body = json!({
            "query": query,
            "per_page": limit,
            "types": ["venue"],
            "geo": { "latitude": lat, "longitude": lng }
        });
        let response = self
            .execute(|c| {
                c.post(&url)
                    .header(CONTENT_TYPE, "application/json")
                    .json(&body)
            })
            .await?;
        read_json_response(response).await
    }

    pub async fn find(
        &self,
        venue_id: i64,
        day: &str,
        party_size: u8,
        lat: f64,
        lng: f64,
    ) -> Result<FindResponse, Error> {
        let url = self.endpoint("/4/find");
        let query = [
            ("venue_id", venue_id.to_string()),
            ("day", day.to_string()),
            ("party_size", party_size.to_string()),
            ("lat", lat.to_string()),
            ("long", lng.to_string()),
        ];
        let response = self.execute(|c| c.get(&url).query(&query)).await?;
        read_json_response(response).await
    }

    pub async fn details_with_commit(
        &self,
        config_id: &ConfigId,
        commit: i32,
    ) -> Result<DetailsResponse, Error> {
        let url = self.endpoint("/3/details");
        let body = json!({
            "config_id": config_id.as_str(),
            "commit": commit,
            "struct_items": [],
        });
        let response = self
            .execute(|c| {
                c.post(&url)
                    .header(CONTENT_TYPE, "application/json")
                    .json(&body)
            })
            .await?;
        read_json_response(response).await
    }

    pub async fn reservations(
        &self,
        resy_token: Option<&ResyToken>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<ReservationLookupResponse, Error> {
        let mut query: Vec<(&str, String)> = Vec::new();
        if let Some(token) = resy_token {
            query.push(("resy_token", token.as_str().to_string()));
        }
        if let Some(limit) = limit {
            query.push(("limit", limit.to_string()));
        }
        if let Some(offset) = offset {
            query.push(("offset", offset.to_string()));
        }

        let url = self.endpoint("/3/user/reservations");
        let response = self.execute(|c| c.get(&url).query(&query)).await?;
        read_json_response(response).await
    }

    pub async fn reservation_by_token(
        &self,
        resy_token: &ResyToken,
    ) -> Result<ReservationLookupResponse, Error> {
        self.reservations(Some(resy_token), None, None).await
    }

    pub async fn cancel(&self, resy_token: &ResyToken) -> Result<CancelResponse, Error> {
        let url = self.endpoint("/3/cancel");
        let form = [("resy_token", resy_token.as_str())];
        let response = self
            .execute(|c| {
                c.post(&url)
                    .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .form(&form)
            })
            .await?;
        read_json_response(response).await
    }

    pub async fn book(
        &self,
        book_token: &BookToken,
        payment_method_id: Option<i64>,
        replace: bool,
        venue_marketing_opt_in: bool,
    ) -> Result<BookResponse, Error> {
        let mut form = vec![("book_token", book_token.as_str().to_string())];
        if let Some(id) = payment_method_id {
            let payment = json!({ "id": id }).to_string();
            form.push(("struct_payment_method", payment));
        }
        form.push(("replace", if replace { "1" } else { "0" }.to_string()));
        form.push((
            "venue_marketing_opt_in",
            if venue_marketing_opt_in { "1" } else { "0" }.to_string(),
        ));

        let url = self.endpoint("/3/book");
        let response = self
            .execute(|c| {
                c.post(&url)
                    .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .form(&form)
            })
            .await?;
        read_json_response(response).await
    }

    /// Run a request, attaching auth headers each attempt. On 401 with stored
    /// credentials, re-authenticate, persist the new token, and retry once.
    async fn execute<F>(&self, build: F) -> Result<Response, Error>
    where
        F: Fn(&reqwest::Client) -> RequestBuilder,
    {
        let response = self.send_with_auth(&build).await?;
        if response.status() != StatusCode::UNAUTHORIZED {
            return Ok(response);
        }
        if !self.try_refresh_auth().await? {
            return Ok(response);
        }
        self.send_with_auth(&build).await
    }

    async fn send_with_auth<F>(&self, build: &F) -> Result<Response, Error>
    where
        F: Fn(&reqwest::Client) -> RequestBuilder,
    {
        let token = self.auth_token.lock().await.clone();
        let mut req = build(&self.http).header(AUTHORIZATION, self.client_auth_header());
        if let Some(token) = token {
            req = req
                .header("x-resy-universal-auth", token.clone())
                .header("x-resy-auth-token", token);
        }
        req.send()
            .await
            .map_err(|e| Error::Api(ApiError::Request(e)))
    }

    async fn try_refresh_auth(&self) -> Result<bool, Error> {
        let Some(creds) = self.credentials.clone() else {
            return Ok(false);
        };

        let auth = self.auth_password(&creds.email, &creds.password).await?;
        let token = auth
            .token
            .clone()
            .ok_or(Error::Api(ApiError::ReauthResponseMissingToken))?;

        *self.auth_token.lock().await = Some(token.clone());

        let mut s = state::load().unwrap_or_default();
        s.auth_token = Some(token);
        if let Some(payment_method_id) = auth.payment_method_id {
            s.payment_method_id = Some(payment_method_id);
        }
        state::save(&s)?;

        Ok(true)
    }

    fn client_auth_header(&self) -> String {
        format!("ResyAPI api_key=\"{}\"", self.client_key)
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
) -> Result<T, Error> {
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|e| Error::Api(ApiError::ReadBody(e)))?;

    let parsed = serde_json::from_str::<Value>(&body)
        .map_err(|e| Error::Api(ApiError::ParseBodyJson(e)))?;

    if !status.is_success() {
        return Err(Error::Api(ApiError::ErrorStatusCode {
            status: status.as_u16(),
            body: parsed,
        }));
    }

    serde_json::from_value::<T>(parsed).map_err(|e| Error::Api(ApiError::DeserializeResponse(e)))
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
        ResyClient::new_with_base_url("test-client-key", "test-auth-token", &server.base_url())
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
            .details_with_commit(&ConfigId("rgs://resy/config-token".to_string()), 0)
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
            .details_with_commit(&ConfigId("rgs://resy/config-token".to_string()), 1)
            .await
            .unwrap();

        mock.assert();
        assert_eq!(
            response.book_token.and_then(|t| t.value),
            Some(BookToken("book-token-xyz".to_string()))
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
            .book(
                &BookToken("book-token-xyz".to_string()),
                Some(31340008),
                false,
                false,
            )
            .await
            .unwrap();

        mock.assert();
        assert_eq!(response.reservation_id, Some(867413540));
        assert_eq!(
            response.resy_token,
            Some(ResyToken("resy-token-abc".to_string()))
        );
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
        let response = client
            .cancel(&ResyToken("resy-token-abc".to_string()))
            .await
            .unwrap();

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
        assert_eq!(err.kind(), "error_status_code");
        assert!(err.to_string().contains("api error 404"));
    }
}
