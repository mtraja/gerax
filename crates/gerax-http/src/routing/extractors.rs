use bytes::Bytes;
use serde::de::DeserializeOwned;
use std::convert::Infallible;
use std::str::FromStr;
use std::sync::Arc;

use super::Context;

pub trait FromContext<S>: Sized {
    type Rejection;

    fn from_context(ctx: &Context<S>) -> Result<Self, Self::Rejection>;
}

use std::fmt;

#[derive(Debug)]
pub enum ExtractError {
    Deserialize(String),
    Missing(String),
    Invalid(String),
}

impl fmt::Display for ExtractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Deserialize(message) => {
                write!(f, "failed to deserialize request data: {message}")
            }

            Self::Missing(name) => {
                write!(f, "missing request parameter: {name}")
            }

            Self::Invalid(message) => {
                write!(f, "invalid request data: {message}")
            }
        }
    }
}

impl From<serde_json::Error> for ExtractError {
    fn from(error: serde_json::Error) -> Self {
        Self::Deserialize(error.to_string())
    }
}

impl From<serde_urlencoded::de::Error> for ExtractError {
    fn from(error: serde_urlencoded::de::Error) -> Self {
        Self::Deserialize(error.to_string())
    }
}

impl std::error::Error for ExtractError {}

pub struct State<S>(pub Arc<S>);

impl<S> FromContext<S> for State<S>
where
    S: Send + Sync + 'static,
{
    type Rejection = Infallible;

    fn from_context(ctx: &Context<S>) -> Result<Self, Self::Rejection> {
        Ok(State(ctx.state()))
    }
}

pub struct Path<T>(pub T);

impl<S, T> FromContext<S> for Path<T>
where
    T: DeserializeOwned,
{
    type Rejection = ExtractError;

    fn from_context(ctx: &Context<S>) -> Result<Self, Self::Rejection> {
        let value = ctx.params().deserialize()?;

        Ok(Path(value))
    }
}

pub struct Query<T>(pub T);
impl<S, T> FromContext<S> for Query<T>
where
    T: DeserializeOwned,
{
    type Rejection = ExtractError;

    fn from_context(ctx: &Context<S>) -> Result<Self, Self::Rejection> {
        let value = serde_urlencoded::from_str(&ctx.request().query)?;

        Ok(Query(value))
    }
}
pub struct Json<T>(pub T);

impl<S, T> FromContext<S> for Json<T>
where
    T: DeserializeOwned,
{
    type Rejection = ExtractError;

    fn from_context(ctx: &Context<S>) -> Result<Self, Self::Rejection> {
        let value = serde_json::from_slice(&ctx.request().body)?;

        Ok(Json(value))
    }
}

pub struct Form<T>(pub T);

impl<S, T> FromContext<S> for Form<T>
where
    T: DeserializeOwned,
{
    type Rejection = ExtractError;

    fn from_context(ctx: &Context<S>) -> Result<Self, Self::Rejection> {
        let value = serde_urlencoded::from_bytes(&ctx.request().body)
            .map_err(|err| ExtractError::Deserialize(err.to_string()))?;

        Ok(Form(value))
    }
}

pub struct Header<T>(pub T);

impl<T> Header<T>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    pub fn from_name<S>(ctx: &Context<S>, header_name: &str) -> Result<Self, ExtractError> {
        let value = ctx
            .request()
            .headers()
            .get(header_name)
            .ok_or_else(|| ExtractError::Missing(header_name.to_string()))?;

        value
            .parse::<T>()
            .map(Header)
            .map_err(|err| ExtractError::Deserialize(err.to_string()))
    }
}

pub struct RawBody(pub Bytes);

impl<S> FromContext<S> for RawBody {
    type Rejection = ExtractError;

    fn from_context(ctx: &Context<S>) -> Result<Self, Self::Rejection> {
        Ok(RawBody(Bytes::from(ctx.request().body.clone())))
    }
}

use super::Request;

impl<S> FromContext<S> for Request {
    type Rejection = Infallible;

    fn from_context(ctx: &Context<S>) -> Result<Self, Self::Rejection> {
        Ok(ctx.request().clone())
    }
}
