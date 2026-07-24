//! Compiled protobuf types and gRPC client/server stubs.
//! All .proto files are compiled at build time via build.rs.
//! Each package gets its own module matching the proto package namespace.

pub mod common {
    include!(concat!(env!("OUT_DIR"), "/common.v1.rs"));
}

pub mod health {
    include!(concat!(env!("OUT_DIR"), "/health.v1.rs"));
}

pub mod operator {
    include!(concat!(env!("OUT_DIR"), "/operator.v1.rs"));
}

pub mod iam {
    include!(concat!(env!("OUT_DIR"), "/iam.v1.rs"));
}

pub mod compliance {
    include!(concat!(env!("OUT_DIR"), "/compliance.v1.rs"));
}

pub mod connector {
    include!(concat!(env!("OUT_DIR"), "/connector.v1.rs"));
}

pub mod orchestration {
    include!(concat!(env!("OUT_DIR"), "/orchestration.v1.rs"));
}

pub mod invoice {
    include!(concat!(env!("OUT_DIR"), "/invoice.v1.rs"));
}

pub mod subscription {
    include!(concat!(env!("OUT_DIR"), "/subscription.v1.rs"));
}

pub mod payment_link {
    include!(concat!(env!("OUT_DIR"), "/payment_link.v1.rs"));
}

pub mod reconciliation {
    include!(concat!(env!("OUT_DIR"), "/reconciliation.v1.rs"));
}

pub mod dispute {
    include!(concat!(env!("OUT_DIR"), "/dispute.v1.rs"));
}

pub mod risk {
    include!(concat!(env!("OUT_DIR"), "/risk.v1.rs"));
}

pub mod ai_assistant {
    include!(concat!(env!("OUT_DIR"), "/ai_assistant.v1.rs"));
}

pub mod document {
    include!(concat!(env!("OUT_DIR"), "/document.v1.rs"));
}

pub mod notification {
    include!(concat!(env!("OUT_DIR"), "/notification.v1.rs"));
}

pub mod analytics {
    include!(concat!(env!("OUT_DIR"), "/analytics.v1.rs"));
}

pub mod saga {
    include!(concat!(env!("OUT_DIR"), "/saga.v1.rs"));
}

pub mod gateway_profile {
    include!(concat!(env!("OUT_DIR"), "/gateway_profile.v1.rs"));
}
