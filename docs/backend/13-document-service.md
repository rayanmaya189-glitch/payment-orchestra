# 13 — document-service (BC-13 Document Management)

MinIO-backed blob storage + metadata. Triggers OCR pipeline.

---

## 1. Domain Model

### AGG-DocumentRecord (Root)

**Identity**: `document_id: Uuid`

**State Entity (SeaORM)**:

```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "document_record")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub document_id: Uuid,
    pub operator_id: Uuid,
    pub uploaded_by: Uuid,
    pub document_type: String, // 'kyb_evidence' | 'settlement_advice' | 'general'
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub minio_key: String,
    pub status: String, // 'uploaded' | 'ocr_processing' | 'ocr_completed' | 'ocr_failed'
    pub ocr_result: Option<String>, // JSON structured extraction
    pub created_at: DateTimeWithTimeZone,
}
```

---

## 2. Commands

### UploadDocument

```rust
pub struct UploadDocumentCommand {
    pub document_type: String,
    pub filename: String,
    pub content_type: String,
    pub body: Vec<u8>,
}
```

**Preconditions**:
- File size <= 10MB (APISEC-001)
- Content type in allowed list

**Produces**: `DocumentUploaded`, triggers async OCR via ai-gateway

### GetDocument

Returns presigned MinIO URL (temporary, 1-hour expiry)

---

## 3. TDD Tests

```rust
#[tokio::test]
async fn test_upload_document_success() {
    let result = handler.handle(UploadDocumentCommand {
        document_type: "kyb_evidence".into(),
        filename: "license.pdf".into(),
        content_type: "application/pdf".into(),
        body: pdf_bytes,
    }).await.unwrap();
    assert_eq!(result.status, "uploaded");
}

#[tokio::test]
async fn test_upload_oversized_document_rejected() {
    let result = handler.handle(UploadDocumentCommand {
        body: vec![0u8; 11 * 1024 * 1024], // 11MB
        ...
    }).await;
    assert!(result.is_err());
}
```
