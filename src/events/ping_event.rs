use bevy::prelude::*;
use std::time::Instant;

/// Event sent when a ping request should be sent to the server
#[derive(Event, Message, Clone, Debug)]
pub struct PingRequestEvent {
    /// The timestamp when the ping was initiated
    pub timestamp: Instant,
}

/// Event sent when a ping response is received from the server
#[derive(Event, Message, Clone, Debug)]
pub struct PingResponseEvent {
    /// The round-trip time in milliseconds
    pub ping_ms: u64,
}

/// Resource to track pending ping requests
#[derive(Resource, Default)]
pub struct PingState {
    /// The timestamp when the last ping was sent
    pub pending_ping_timestamp: Option<Instant>,
    /// The last measured ping in milliseconds
    pub last_ping_ms: Option<u64>,
}
