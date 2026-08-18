use std::collections::HashMap;
use std::sync::Arc;
use super::pathparams::PathParams;
use super::extensions::Extensions;
use super::Request;

pub struct Context<State> {
    pub state: Arc<State>,
    pub request: Request,
    pub params: PathParams,
    pub extensions: Extensions,
}

impl<State> Clone for Context<State> {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            request: self.request.clone(),
            params: self.params.clone(),
            extensions: self.extensions.clone(),
        }
    }
}

impl<State> Context<State> {
    pub fn new(state: Arc<State>, request: Request) -> Self {
        Self {
            state,
            params: PathParams::new(HashMap::new()),
            extensions: Extensions::new(),
            request,
        }
    }

    pub fn state(&self) -> Arc<State> {
        Arc::clone(&self.state)
    }

    pub fn params(&self) -> &PathParams {
        &self.params
    }

    pub fn params_mut(&mut self) -> &mut PathParams {
        &mut self.params
    }

    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }

    pub fn request(&self) -> &Request {
        &self.request
    }

    pub fn request_mut(&mut self) -> &mut Request {
        &mut self.request
    }
}
