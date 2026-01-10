use crate::{
    global::dxb_block::IncomingSection, values::core_values::endpoint::Endpoint,
};
use core::fmt::Display;
use core::fmt::Formatter;
use core::time::Duration;
#[derive(Default, PartialEq, Debug)]
pub enum ResponseResolutionStrategy {
    /// Promise.allSettled
    /// - For know fixed receivers:
    ///   return after all known sends are finished (either success or error
    ///   if block could not be sent / timed out)
    /// - For unknown receiver count:
    ///   return after timeout
    #[default]
    ReturnAfterAllSettled,

    /// Promise.all
    /// - For know fixed receivers:
    ///   return after all known sends are finished successfully
    ///   return immediately if one send fails early (e.g. endpoint not reachable)
    /// - For unknown receiver count:
    ///   return after timeout
    ///
    ReturnOnAnyError,

    /// Promise.any
    /// Return after first successful response received
    ReturnOnFirstResponse,

    /// Promise.race
    /// Return after first response received (success or error)
    ReturnOnFirstResult,
}

#[derive(Default, Debug)]
pub enum ResponseTimeout {
    #[default]
    Default,
    Custom(Duration),
}

impl ResponseTimeout {
    pub fn unwrap_or_default(self, default: Duration) -> Duration {
        match self {
            ResponseTimeout::Default => default,
            ResponseTimeout::Custom(timeout) => timeout,
        }
    }
}

#[derive(Default, Debug)]
pub struct ResponseOptions {
    pub resolution_strategy: ResponseResolutionStrategy,
    pub timeout: ResponseTimeout,
}

impl ResponseOptions {
    pub fn new_with_resolution_strategy(
        resolution_strategy: ResponseResolutionStrategy,
    ) -> Self {
        Self {
            resolution_strategy,
            ..ResponseOptions::default()
        }
    }

    pub fn new_with_timeout(timeout: Duration) -> Self {
        Self {
            timeout: ResponseTimeout::Custom(timeout),
            ..ResponseOptions::default()
        }
    }
}

#[derive(Debug)]
pub enum Response {
    ExactResponse(Endpoint, IncomingSection),
    ResolvedResponse(Endpoint, IncomingSection),
    UnspecifiedResponse(IncomingSection),
}

impl Response {
    pub fn take_incoming_section(self) -> IncomingSection {
        match self {
            Response::ExactResponse(_, section) => section,
            Response::ResolvedResponse(_, section) => section,
            Response::UnspecifiedResponse(section) => section,
        }
    }
}

#[derive(Debug)]
pub enum ResponseError {
    NoResponseAfterTimeout(Endpoint, Duration),
    NotReachable(Endpoint),
    EarlyAbort(Endpoint),
}

impl Display for ResponseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            ResponseError::NoResponseAfterTimeout(endpoint, duration) => {
                core::write!(
                    f,
                    "No response after timeout ({}s) for endpoint {}",
                    duration.as_secs(),
                    endpoint
                )
            }
            ResponseError::NotReachable(endpoint) => {
                core::write!(f, "Endpoint {endpoint} is not reachable")
            }
            ResponseError::EarlyAbort(endpoint) => {
                core::write!(f, "Early abort for endpoint {endpoint}")
            }
        }
    }
}
