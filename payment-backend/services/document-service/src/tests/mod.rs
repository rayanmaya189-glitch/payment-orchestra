//! Document Management TDD tests — BC-13
//!
//! Spec tests:
//! - test_upload_document_success: basic upload
//! - test_upload_oversized_document_rejected: 11MB → error
//! - test_upload_unsupported_content_type_rejected: invalid MIME
//! - test_ocr_lifecycle: uploaded → processing → completed
//! - test_ocr_failure: uploaded → processing → failed
//! - test_start_ocr_on_uploaded_only: invalid transition
//! - test_get_document: query by ID
//! - test_delete_document: delete
//! - test_find_by_type: query by category

use crate::commands::*;
use crate::domain::*;
use crate::pipeline::*;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup() -> DocumentPipeline {
    DocumentPipeline::new()
}

fn upload_cmd() -> UploadDocumentCommand {
    UploadDocumentCommand {
        operator_id: Uuid::now_v7(),
        uploaded_by: Uuid::now_v7(),
        category: DocumentCategory::KybEvidence,
        filename: "business_license.pdf".into(),
        content_type: "application/pdf".into(),
        body: vec![0u8; 1024], // 1KB
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_upload_document_success() {
    let pipeline = setup();
    let record = pipeline.api.upload_document(upload_cmd()).await.unwrap();

    assert_eq!(record.status, DocumentStatus::Uploaded);
    assert_eq!(record.category, DocumentCategory::KybEvidence);
    assert_eq!(record.filename, "business_license.pdf");
    assert_eq!(record.content_type, "application/pdf");
    assert_eq!(record.size_bytes, 1024);
    assert!(record.storage_key.contains("business_license.pdf"));
}

#[tokio::test]
async fn test_upload_oversized_document_rejected() {
    let pipeline = setup();
    let cmd = UploadDocumentCommand {
        body: vec![0u8; 11 * 1024 * 1024], // 11MB
        ..upload_cmd()
    };
    let result = pipeline.api.upload_document(cmd).await;
    assert!(result.is_err(), "Oversized document should be rejected");
}

#[tokio::test]
async fn test_upload_unsupported_content_type_rejected() {
    let pipeline = setup();
    let cmd = UploadDocumentCommand {
        content_type: "application/octet-stream".into(),
        ..upload_cmd()
    };
    let result = pipeline.api.upload_document(cmd).await;
    assert!(result.is_err(), "Unsupported content type should be rejected");
}

#[tokio::test]
async fn test_ocr_lifecycle_uploaded_to_completed() {
    let pipeline = setup();
    let record = pipeline.api.upload_document(upload_cmd()).await.unwrap();

    // Start OCR
    let processing = pipeline
        .api
        .start_ocr(StartOcrCommand {
            document_id: record.document_id,
        })
        .await
        .unwrap();
    assert_eq!(processing.status, DocumentStatus::OcrProcessing);

    // Complete OCR
    let completed = pipeline
        .api
        .complete_ocr(CompleteOcrCommand {
            document_id: record.document_id,
            ocr_result: r#"{"text": "License #12345", "confidence": 0.95}"#.into(),
        })
        .await
        .unwrap();
    assert_eq!(completed.status, DocumentStatus::OcrCompleted);
    assert!(completed.ocr_result.unwrap().contains("License #12345"));
}

#[tokio::test]
async fn test_ocr_failure() {
    let pipeline = setup();
    let record = pipeline.api.upload_document(upload_cmd()).await.unwrap();

    // Start OCR
    pipeline
        .api
        .start_ocr(StartOcrCommand {
            document_id: record.document_id,
        })
        .await
        .unwrap();

    // Fail OCR
    let failed = pipeline
        .api
        .fail_ocr(FailOcrCommand {
            document_id: record.document_id,
            error: "Image quality too low".into(),
        })
        .await
        .unwrap();
    assert_eq!(failed.status, DocumentStatus::OcrFailed);
    assert!(failed.ocr_result.unwrap().contains("Image quality too low"));
}

#[tokio::test]
async fn test_start_ocr_on_uploaded_only() {
    let pipeline = setup();
    let record = pipeline.api.upload_document(upload_cmd()).await.unwrap();

    // Complete OCR without starting processing (invalid transition)
    let result = pipeline
        .api
        .complete_ocr(CompleteOcrCommand {
            document_id: record.document_id,
            ocr_result: "test".into(),
        })
        .await;
    assert!(
        result.is_err(),
        "Completing OCR without starting should fail"
    );

    // Delete should work
    let _ = pipeline
        .api
        .delete_document(DeleteDocumentCommand {
            document_id: record.document_id,
        })
        .await;
}

#[tokio::test]
async fn test_get_document_by_id() {
    let pipeline = setup();
    let record = pipeline.api.upload_document(upload_cmd()).await.unwrap();

    let fetched = pipeline.api.get_document(record.document_id).await.unwrap();
    assert_eq!(fetched.document_id, record.document_id);
    assert_eq!(fetched.filename, "business_license.pdf");
}

#[tokio::test]
async fn test_delete_document() {
    let pipeline = setup();
    let record = pipeline.api.upload_document(upload_cmd()).await.unwrap();

    // Delete should succeed
    pipeline
        .api
        .delete_document(DeleteDocumentCommand {
            document_id: record.document_id,
        })
        .await
        .unwrap();

    // Query should fail
    let result = pipeline.api.get_document(record.document_id).await;
    assert!(result.is_err(), "Deleted document should not be found");
}

#[tokio::test]
async fn test_find_by_type() {
    let pipeline = setup();
    let operator_id = Uuid::now_v7();

    // Upload KYB evidence
    let cmd1 = UploadDocumentCommand {
        operator_id,
        category: DocumentCategory::KybEvidence,
        ..upload_cmd()
    };
    pipeline.api.upload_document(cmd1).await.unwrap();

    // Upload settlement advice
    let cmd2 = UploadDocumentCommand {
        operator_id,
        category: DocumentCategory::SettlementAdvice,
        filename: "settlement.csv".into(),
        content_type: "application/csv".into(),
        ..upload_cmd()
    };
    pipeline.api.upload_document(cmd2).await.unwrap();

    // Find by type should return 1
    let kyb_docs = pipeline
        .api
        .find_by_type(operator_id, "kyb_evidence")
        .await
        .unwrap();
    assert_eq!(kyb_docs.len(), 1);

    let settlement_docs = pipeline
        .api
        .find_by_type(operator_id, "settlement_advice")
        .await
        .unwrap();
    assert_eq!(settlement_docs.len(), 1);
}
