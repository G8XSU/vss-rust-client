use ::prost::Message;
use reqwest;
use reqwest::Client;
use std::error::Error;
use std::io::Read;

use crate::error::VssError;
use crate::types::{
	DeleteObjectRequest, DeleteObjectResponse, GetObjectRequest, GetObjectResponse, ListKeyVersionsRequest,
	ListKeyVersionsResponse, PutObjectRequest, PutObjectResponse,
};
use crate::util::retry::{retry, RetryPolicy};

/// Thin-client to access a hosted instance of Versioned Storage Service (VSS).
/// The provided [`VssClient`] API is minimalistic and is congruent to the VSS server-side API.
#[derive(Clone)]
pub struct VssClient<R>
where
	R: RetryPolicy<E = VssError>,
{
	base_url: String,
	client: Client,
	retry_policy: R,
}

impl<R: RetryPolicy<E = VssError>> VssClient<R> {
	/// Constructs a [`VssClient`] using `base_url` as the VSS server endpoint.
	pub fn new(base_url: &str, retry_policy: R) -> Self {
		let client = Client::new();
		Self::from_client(base_url, client, retry_policy)
	}

	/// Constructs a [`VssClient`] from a given [`reqwest::Client`], using `base_url` as the VSS server endpoint.
	pub fn from_client(base_url: &str, client: Client, retry_policy: R) -> Self {
		Self { base_url: String::from(base_url), client, retry_policy }
	}

	/// Returns the underlying base URL.
	pub fn base_url(&self) -> &str {
		&self.base_url
	}

	/// Fetches a value against a given `key` in `request`.
	/// Makes a service call to the `GetObject` endpoint of the VSS server.
	/// For API contract/usage, refer to docs for [`GetObjectRequest`] and [`GetObjectResponse`].
	pub async fn get_object(&self, request: &GetObjectRequest) -> Result<GetObjectResponse, VssError> {
		let url = format!("{}/getObject", self.base_url);

		let request_body = request.encode_to_vec();
		let raw_response = self.client.post(url).body(request_body).send().await?;
		let status = raw_response.status();
		let payload = raw_response.bytes().await?;

		if status.is_success() {
			let response = GetObjectResponse::decode(&payload[..])?;

			if response.value.is_none() {
				return Err(VssError::InternalServerError(
					"VSS Server API Violation, expected value in GetObjectResponse but found none".to_string(),
				));
			}

			Ok(response)
		} else {
			Err(VssError::new(status, payload))
		}
	}

	/// Writes multiple [`PutObjectRequest::transaction_items`] as part of a single transaction.
	/// Makes a service call to the `PutObject` endpoint of the VSS server, with multiple items.
	/// Items in the `request` are written in a single all-or-nothing transaction.
	/// For API contract/usage, refer to docs for [`PutObjectRequest`] and [`PutObjectResponse`].
	pub async fn put_object(&self, request: &PutObjectRequest) -> Result<PutObjectResponse, VssError> {
		retry(
			|| async {
				let url = format!("{}/putObjects", self.base_url);

				let request_body = request.encode_to_vec();
				let response_raw = self.client.post(&url).body(request_body).send().await?;
				let status = response_raw.status();
				let payload = response_raw.bytes().await?;

				if status.is_success() {
					let response = PutObjectResponse::decode(&payload[..])?;
					Ok(response)
				} else {
					Err(VssError::new(status, payload))
				}
			},
			&self.retry_policy,
		)
		.await
	}

	/// Deletes the given `key` and `value` in `request`.
	/// Makes a service call to the `DeleteObject` endpoint of the VSS server.
	/// For API contract/usage, refer to docs for [`DeleteObjectRequest`] and [`DeleteObjectResponse`].
	pub async fn delete_object(&self, request: &DeleteObjectRequest) -> Result<DeleteObjectResponse, VssError> {
		let url = format!("{}/deleteObject", self.base_url);

		let request_body = request.encode_to_vec();
		let response_raw = self.client.post(url).body(request_body).send().await?;
		let status = response_raw.status();
		let payload = response_raw.bytes().await?;

		if status.is_success() {
			let response = DeleteObjectResponse::decode(&payload[..])?;
			Ok(response)
		} else {
			Err(VssError::new(status, payload))
		}
	}

	/// Lists keys and their corresponding version for a given [`ListKeyVersionsRequest::store_id`].
	/// Makes a service call to the `ListKeyVersions` endpoint of the VSS server.
	/// For API contract/usage, refer to docs for [`ListKeyVersionsRequest`] and [`ListKeyVersionsResponse`].
	pub async fn list_key_versions(
		&self, request: &ListKeyVersionsRequest,
	) -> Result<ListKeyVersionsResponse, VssError> {
		let url = format!("{}/listKeyVersions", self.base_url);

		let request_body = request.encode_to_vec();
		let response_raw = self.client.post(url).body(request_body).send().await?;
		let status = response_raw.status();
		let payload = response_raw.bytes().await?;

		if status.is_success() {
			let response = ListKeyVersionsResponse::decode(&payload[..])?;
			Ok(response)
		} else {
			Err(VssError::new(status, payload))
		}
	}
}
