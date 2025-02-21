use eth2::ok_or_error;
use eth2::types::builder_bid::SignedBuilderBid;
use eth2::types::{
    BlindedPayload, EthSpec, ExecPayload, ExecutionBlockHash, ExecutionPayload,
    ForkVersionedResponse, PublicKeyBytes, SignedBeaconBlock, SignedValidatorRegistrationData,
    Slot,
};
pub use eth2::Error;
use reqwest::{IntoUrl, Response};
use sensitive_url::SensitiveUrl;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::time::Duration;

pub const DEFAULT_GET_HEADER_TIMEOUT_MILLIS: u64 = 500;

#[derive(Clone)]
pub struct Timeouts {
    get_header: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            get_header: Duration::from_millis(DEFAULT_GET_HEADER_TIMEOUT_MILLIS),
        }
    }
}

#[derive(Clone)]
pub struct BuilderHttpClient {
    client: reqwest::Client,
    server: SensitiveUrl,
    timeouts: Timeouts,
}

impl BuilderHttpClient {
    pub fn new(server: SensitiveUrl) -> Result<Self, Error> {
        Ok(Self {
            client: reqwest::Client::new(),
            server,
            timeouts: Timeouts::default(),
        })
    }

    pub fn new_with_timeouts(server: SensitiveUrl, timeouts: Timeouts) -> Result<Self, Error> {
        Ok(Self {
            client: reqwest::Client::new(),
            server,
            timeouts,
        })
    }

    async fn get<T: DeserializeOwned, U: IntoUrl>(&self, url: U) -> Result<T, Error> {
        self.get_response_with_timeout(url, None)
            .await?
            .json()
            .await
            .map_err(Error::Reqwest)
    }

    async fn get_with_timeout<T: DeserializeOwned, U: IntoUrl>(
        &self,
        url: U,
        timeout: Duration,
    ) -> Result<T, Error> {
        self.get_response_with_timeout(url, Some(timeout))
            .await?
            .json()
            .await
            .map_err(Error::Reqwest)
    }

    /// Perform a HTTP GET request, returning the `Response` for further processing.
    async fn get_response_with_timeout<U: IntoUrl>(
        &self,
        url: U,
        timeout: Option<Duration>,
    ) -> Result<Response, Error> {
        let mut builder = self.client.get(url);
        if let Some(timeout) = timeout {
            builder = builder.timeout(timeout);
        }
        let response = builder.send().await.map_err(Error::Reqwest)?;
        ok_or_error(response).await
    }

    /// Generic POST function supporting arbitrary responses and timeouts.
    async fn post_generic<T: Serialize, U: IntoUrl>(
        &self,
        url: U,
        body: &T,
        timeout: Option<Duration>,
    ) -> Result<Response, Error> {
        let mut builder = self.client.post(url);
        if let Some(timeout) = timeout {
            builder = builder.timeout(timeout);
        }
        let response = builder.json(body).send().await?;
        ok_or_error(response).await
    }

    async fn post_with_raw_response<T: Serialize, U: IntoUrl>(
        &self,
        url: U,
        body: &T,
    ) -> Result<Response, Error> {
        let response = self
            .client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(Error::Reqwest)?;
        ok_or_error(response).await
    }

    /// `POST /eth/v1/builder/validators`
    pub async fn post_builder_validators(
        &self,
        validator: &[SignedValidatorRegistrationData],
    ) -> Result<(), Error> {
        let mut path = self.server.full.clone();

        path.path_segments_mut()
            .map_err(|()| Error::InvalidUrl(self.server.clone()))?
            .push("eth")
            .push("v1")
            .push("builder")
            .push("validators");

        self.post_generic(path, &validator, None).await?;
        Ok(())
    }

    /// `POST /eth/v1/builder/blinded_blocks`
    pub async fn post_builder_blinded_blocks<E: EthSpec>(
        &self,
        blinded_block: &SignedBeaconBlock<E, BlindedPayload<E>>,
    ) -> Result<ForkVersionedResponse<ExecutionPayload<E>>, Error> {
        let mut path = self.server.full.clone();

        path.path_segments_mut()
            .map_err(|()| Error::InvalidUrl(self.server.clone()))?
            .push("eth")
            .push("v1")
            .push("builder")
            .push("blinded_blocks");

        Ok(self
            .post_with_raw_response(path, &blinded_block)
            .await?
            .json()
            .await?)
    }

    /// `GET /eth/v1/builder/header`
    pub async fn get_builder_header<E: EthSpec, Payload: ExecPayload<E>>(
        &self,
        slot: Slot,
        parent_hash: ExecutionBlockHash,
        pubkey: &PublicKeyBytes,
    ) -> Result<ForkVersionedResponse<SignedBuilderBid<E, Payload>>, Error> {
        let mut path = self.server.full.clone();

        path.path_segments_mut()
            .map_err(|()| Error::InvalidUrl(self.server.clone()))?
            .push("eth")
            .push("v1")
            .push("builder")
            .push("header")
            .push(slot.to_string().as_str())
            .push(format!("{parent_hash:?}").as_str())
            .push(pubkey.as_hex_string().as_str());

        self.get_with_timeout(path, self.timeouts.get_header).await
    }

    /// `GET /eth/v1/builder/status`
    pub async fn get_builder_status<E: EthSpec>(&self) -> Result<(), Error> {
        let mut path = self.server.full.clone();

        path.path_segments_mut()
            .map_err(|()| Error::InvalidUrl(self.server.clone()))?
            .push("eth")
            .push("v1")
            .push("builder")
            .push("status");

        self.get(path).await
    }
}
