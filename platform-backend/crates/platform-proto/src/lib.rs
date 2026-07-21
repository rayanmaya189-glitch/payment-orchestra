#![allow(dead_code, unused_imports)]
//! Generated gRPC protobuf types and service stubs.
//!
//! This crate contains auto-generated Rust code from the `.proto` definitions
//! in the `proto/` directory. Do not edit generated code manually.

pub mod common {
    tonic::include_proto!("common");
}

pub mod orchestration {
    tonic::include_proto!("orchestration");
}

pub mod iam {
    tonic::include_proto!("iam");
}

pub mod connector {
    tonic::include_proto!("connector");
}

pub mod compliance {
    tonic::include_proto!("compliance");
}

pub mod operator {
    tonic::include_proto!("operator");
}

pub mod reconciliation {
    tonic::include_proto!("reconciliation");
}

pub mod invoice {
    tonic::include_proto!("invoice");
}

pub mod subscription {
    tonic::include_proto!("subscription");
}

pub mod dispute {
    tonic::include_proto!("dispute");
}

pub mod risk {
    tonic::include_proto!("risk");
}

pub mod notification {
    tonic::include_proto!("notification");
}

pub mod analytics {
    tonic::include_proto!("analytics");
}

pub mod saga {
    tonic::include_proto!("saga");
}

pub mod ai_assistant {
    tonic::include_proto!("ai_assistant");
}

pub mod document {
    tonic::include_proto!("document");
}
