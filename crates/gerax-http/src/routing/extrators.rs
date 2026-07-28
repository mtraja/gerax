use serde::de::DeserializeOwned;
use std::convert::Infallible;
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
