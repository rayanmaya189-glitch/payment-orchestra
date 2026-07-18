# 20 — gRPC Proto Definitions

All service gRPC contracts. Generated from `.proto` files. Each service owns its own proto package.

---

## 1. Shared Types (common.v1)

```protobuf
syntax = "proto3";
package common.v1;

message Money {
  int64 amount_minor_units = 1;
  string currency_code = 2;
}

message PaginationRequest {
  string cursor = 1;
  uint32 limit = 2;
}

message PaginationResponse {
  string next_cursor = 1;
  bool has_more = 2;
}

enum InternalErrorCode {
  INTERNAL_ERROR_CODE_UNSPECIFIED = 0;
  TRANSIENT_FAILURE = 1;
  PERMANENT_FAILURE = 2;
  DEGRADED_MODE = 3;
  RATE_LIMITED = 4;
  AUTHORIZATION_DENIED = 5;
  VALIDATION_ERROR = 6;
  UNAVAILABLE = 7;
}
```

---

## 2. orchestration-service (orchestration.v1)

```protobuf
syntax = "proto3";
package orchestration.v1;
import "common.v1/money.proto";

service OrchestrationService {
  rpc CreatePaymentIntent(CreatePaymentIntentRequest) returns (CreatePaymentIntentResponse);
  rpc AuthorizePaymentIntent(AuthorizePaymentIntentRequest) returns (AuthorizePaymentIntentResponse);
  rpc CapturePaymentIntent(CapturePaymentIntentRequest) returns (CapturePaymentIntentResponse);
  rpc VoidPaymentIntent(VoidPaymentIntentRequest) returns (VoidPaymentIntentResponse);
  rpc RefundPaymentIntent(RefundPaymentIntentRequest) returns (RefundPaymentIntentResponse);
  rpc GetPaymentIntent(GetPaymentIntentRequest) returns (PaymentIntentView);
  rpc ActivateRoutingPolicy(ActivateRoutingPolicyRequest) returns (ActivateRoutingPolicyResponse);
  rpc GetRoutingPolicy(GetRoutingPolicyRequest) returns (RoutingPolicyView);
}

message CreatePaymentIntentRequest {
  common.v1.Money amount = 1;
  string idempotency_key = 2;
  string purpose = 3; // 'payment' | 'card_verification'
  string metadata_json = 4;
}

message CreatePaymentIntentResponse {
  string payment_intent_id = 1;
  string status = 2;
}

message AuthorizePaymentIntentRequest {
  string payment_intent_id = 1;
  string payment_method_token_id = 2;
}

message AuthorizePaymentIntentResponse {
  string status = 1;
  repeated RoutingAttemptResult attempts = 2;
}

message RoutingAttemptResult {
  string acquirer_connector_id = 1;
  bool approved = 2;
  string normalized_decline_reason = 3;
  uint32 latency_ms = 4;
  string acquirer_reference = 5;
}

message CapturePaymentIntentRequest {
  string payment_intent_id = 1;
  common.v1.Money amount = 2; // optional, null = full capture
}

message CapturePaymentIntentResponse {
  string status = 1;
  common.v1.Money captured_amount = 2;
}

message VoidPaymentIntentRequest {
  string payment_intent_id = 1;
}

message VoidPaymentIntentResponse {
  string status = 1;
}

message RefundPaymentIntentRequest {
  string payment_intent_id = 1;
  common.v1.Money amount = 2;
}

message RefundPaymentIntentResponse {
  string status = 1;
  string refund_id = 2;
}

message GetPaymentIntentRequest {
  string payment_intent_id = 1;
}

message PaymentIntentView {
  string payment_intent_id = 1;
  string status = 2;
  common.v1.Money requested_amount = 3;
  common.v1.Money authorized_amount = 4;
  common.v1.Money captured_amount = 5;
  common.v1.Money refunded_amount = 6;
  string currency = 7;
  string created_at = 8;
}

message ActivateRoutingPolicyRequest {
  repeated RoutingRule rules = 1;
  FailoverConfig failover_config = 2;
}

message RoutingRule {
  string acquirer_link_id = 1;
  uint32 priority = 2;
  string condition_json = 3;
}

message FailoverConfig {
  repeated string retryable_decline_codes = 1;
  uint32 max_hops = 2;
  uint32 latency_budget_ms = 3;
}

message ActivateRoutingPolicyResponse {
  string routing_policy_id = 1;
  int32 version = 2;
  string status = 3;
}
```

---

## 3. iam-service (iam.v1)

```protobuf
syntax = "proto3";
package iam.v1;
import "common.v1/money.proto";

service IamService {
  rpc Authenticate(AuthenticateRequest) returns (AuthenticateResponse);
  rpc IssueToken(IssueTokenRequest) returns (IssueTokenResponse);
  rpc ValidatePermission(ValidatePermissionRequest) returns (ValidatePermissionResponse);
  rpc CreateApiKey(CreateApiKeyRequest) returns (CreateApiKeyResponse);
  rpc RevokeApiKey(RevokeApiKeyRequest) returns (RevokeApiKeyResponse);
  rpc ListApiKeys(ListApiKeysRequest) returns (ListApiKeysResponse);
}

message AuthenticateRequest {
  string email = 1;
  string password = 2;
  string ip_address = 3;
  string user_agent = 4;
}

message AuthenticateResponse {
  string access_token = 1;
  string refresh_token = 2;
  int64 expires_in_seconds = 3;
  bool mfa_required = 4;
}

message ValidatePermissionRequest {
  string principal_id = 1;
  string resource = 2;
  string action = 3;
  string context_json = 4;
}

message ValidatePermissionResponse {
  bool allowed = 1;
  bool requires_maker_checker = 2;
  string denial_reason = 3;
}

message CreateApiKeyRequest {
  string name = 1;
  repeated string scopes = 2;
  repeated string acquirer_link_ids = 3;
  int32 expires_in_days = 4;
}

message CreateApiKeyResponse {
  string api_key_id = 1;
  string api_key_secret = 2; // only returned on creation
}
```

---

## 4. connector-gateway (connector.v1)

```protobuf
syntax = "proto3";
package connector.v1;
import "common.v1/money.proto";

service ConnectorGateway {
  rpc Authorize(AuthorizeRequest) returns (AuthorizeResponse);
  rpc Capture(CaptureRequest) returns (CaptureResponse);
  rpc Void(VoidRequest) returns (VoidResponse);
  rpc Refund(RefundRequest) returns (RefundResponse);
  rpc StatusCheck(StatusCheckRequest) returns (StatusCheckResponse);
  rpc GetCapabilities(GetCapabilitiesRequest) returns (CapabilitiesResponse);
}

message AuthorizeRequest {
  string connector_id = 1;
  string payment_method_token = 2;
  common.v1.Money amount = 3;
  string idempotency_key = 4;
  string card_scheme = 5;
}

message AuthorizeResponse {
  string status = 1; // approved | declined | requires_3ds | partial_approval
  string acquirer_reference = 2;
  string decline_reason = 3;
  common.v1.Money approved_amount = 4;
  ThreeDsData three_ds_data = 5;
  uint32 latency_ms = 6;
}

message ThreeDsData {
  string three_ds_url = 1;
  string pareq = 2;
  string md = 3;
}

message CaptureRequest {
  string connector_id = 1;
  string acquirer_reference = 2;
  common.v1.Money amount = 3;
}

message CaptureResponse {
  bool success = 1;
  string acquirer_reference = 2;
}

message VoidRequest {
  string connector_id = 1;
  string acquirer_reference = 2;
}

message VoidResponse {
  bool success = 1;
}

message RefundRequest {
  string connector_id = 1;
  string acquirer_reference = 2;
  common.v1.Money amount = 3;
}

message RefundResponse {
  bool success = 1;
  string refund_reference = 2;
}

message StatusCheckRequest {
  string connector_id = 1;
  string acquirer_reference = 2;
}

message StatusCheckResponse {
  string status = 1;
  common.v1.Money amount = 2;
}

message GetCapabilitiesRequest {
  string connector_id = 1;
}

message CapabilitiesResponse {
  bool supports_partial_capture = 1;
  bool supports_partial_refund = 2;
  bool supports_native_idempotency_key = 3;
  bool supports_webhook_settlement = 4;
  bool supports_realtime_status_check = 5;
  repeated string supported_card_schemes = 6;
  repeated string supported_currencies = 7;
  string settlement_format = 8;
}
```

---

## 5. reconciliation-service (reconciliation.v1)

```protobuf
syntax = "proto3";
package reconciliation.v1;
import "common.v1/money.proto";

service ReconciliationService {
  rpc GetReconciliationExceptions(GetExceptionsRequest) returns (GetExceptionsResponse);
  rpc ResolveException(ResolveExceptionRequest) returns (ResolveExceptionResponse);
  rpc GetSettlementBatches(GetBatchesRequest) returns (GetBatchesResponse);
}

message GetExceptionsRequest {
  string operator_id = 1;
  string status = 2; // unmatched | amount_mismatch | duplicate_reference
  string cursor = 3;
  uint32 limit = 4;
}

message GetExceptionsResponse {
  repeated ReconciliationException exceptions = 1;
  common.v1.PaginationResponse pagination = 2;
}

message ReconciliationException {
  string exception_id = 1;
  string settlement_record_id = 2;
  string payment_intent_id = 3;
  common.v1.Money amount = 4;
  string status = 5;
  string classification = 6;
  string detected_at = 7;
}

message ResolveExceptionRequest {
  string exception_id = 1;
  string resolution = 2; // linked | flagged_discrepancy | orphan
  string payment_intent_id = 3;
  string note = 4;
}

message ResolveExceptionResponse {
  string status = 1;
}
```

---

## 6. invoice-service (invoice.v1)

```protobuf
syntax = "proto3";
package invoice.v1;
import "common.v1/money.proto";

service InvoiceService {
  rpc CreateInvoice(CreateInvoiceRequest) returns (CreateInvoiceResponse);
  rpc GetInvoice(GetInvoiceRequest) returns (InvoiceView);
  rpc ListInvoices(ListInvoicesRequest) returns (ListInvoicesResponse);
  rpc SendInvoice(SendInvoiceRequest) returns (SendInvoiceResponse);
  rpc CancelInvoice(CancelInvoiceRequest) returns (CancelInvoiceResponse);
}

message CreateInvoiceRequest {
  string order_reference = 1;
  repeated InvoiceLineItem line_items = 2;
  string due_date = 3;
  string recipient_email = 4;
}

message InvoiceLineItem {
  string description = 1;
  int64 amount_minor_units = 2;
}

message CreateInvoiceResponse {
  string invoice_id = 1;
  string status = 2;
}

message InvoiceView {
  string invoice_id = 1;
  string status = 2;
  common.v1.Money total_amount = 3;
  common.v1.Money paid_amount = 4;
  string due_date = 5;
  repeated string payment_intent_ids = 6;
}
```

---

## 7. subscription-service (subscription.v1)

```protobuf
syntax = "proto3";
package subscription.v1;
import "common.v1/money.proto";

service SubscriptionService {
  rpc CreateSubscription(CreateSubscriptionRequest) returns (CreateSubscriptionResponse);
  rpc GetSubscription(GetSubscriptionRequest) returns (SubscriptionView);
  rpc CancelSubscription(CancelSubscriptionRequest) returns (CancelSubscriptionResponse);
  rpc PauseSubscription(PauseSubscriptionRequest) returns (PauseSubscriptionResponse);
  rpc ResumeSubscription(ResumeSubscriptionRequest) returns (ResumeSubscriptionResponse);
}

message CreateSubscriptionRequest {
  string customer_id = 1;
  string plan_id = 2;
  common.v1.Money amount = 3;
  string billing_interval = 4; // 'monthly' | 'weekly' | 'yearly'
  string payment_method_token_id = 5;
}

message CreateSubscriptionResponse {
  string subscription_id = 1;
  string status = 2;
}

message SubscriptionView {
  string subscription_id = 1;
  string status = 2;
  string current_period_start = 3;
  string current_period_end = 4;
  int32 dunning_retry_count = 5;
}
```

---

## 8. Other Services (abbreviated)

```protobuf
// compliance-service (compliance.v1)
service ComplianceService {
  rpc SubmitKybEvidence(SubmitKybEvidenceRequest) returns (SubmitKybEvidenceResponse);
  rpc ReviewKybCase(ReviewKybCaseRequest) returns (ReviewKybCaseResponse);
  rpc GetAmlAlerts(GetAmlAlertsRequest) returns (GetAmlAlertsResponse);
  rpc GenerateSar(GenerateSarRequest) returns (GenerateSarResponse);
}

// dispute-service (dispute.v1)
service DisputeService {
  rpc RecordChargeback(RecordChargebackRequest) returns (RecordChargebackResponse);
  rpc SubmitRepresentment(SubmitRepresentmentRequest) returns (SubmitRepresentmentResponse);
  rpc ResolveChargeback(ResolveChargebackRequest) returns (ResolveChargebackResponse);
  rpc GetChargebackCase(GetChargebackCaseRequest) returns (ChargebackCaseView);
}

// risk-service (risk.v1)
service RiskService {
  rpc AssessRisk(AssessRiskRequest) returns (AssessRiskResponse);
}

// ai-assistant-service (ai_assistant.v1)
service AiAssistantService {
  rpc AskAssistant(AskAssistantRequest) returns (AskAssistantResponse);
  rpc GetSessionHistory(GetSessionHistoryRequest) returns (GetSessionHistoryResponse);
}

// document-service (document.v1)
service DocumentService {
  rpc UploadDocument(UploadDocumentRequest) returns (UploadDocumentResponse);
  rpc GetDocumentUrl(GetDocumentUrlRequest) returns (GetDocumentUrlResponse);
}

// notification-service (notification.v1)
service NotificationService {
  rpc SendNotification(SendNotificationRequest) returns (SendNotificationResponse);
  rpc GetDeliveryStatus(GetDeliveryStatusRequest) returns (GetDeliveryStatusResponse);
}

// analytics-service (analytics.v1)
service AnalyticsService {
  rpc GetAuthorizationRates(GetAuthRatesRequest) returns (GetAuthRatesResponse);
  rpc GetDeclineReasons(GetDeclineReasonsRequest) returns (GetDeclineReasonsResponse);
  rpc GetSettlementStatus(GetSettlementStatusRequest) returns (GetSettlementStatusResponse);
  rpc GetFeeAnalysis(GetFeeAnalysisRequest) returns (GetFeeAnalysisResponse);
  rpc GetChargebackTrends(GetChargebackTrendsRequest) returns (GetChargebackTrendsResponse);
}

// saga-coordinator (saga.v1)
service SagaService {
  rpc GetSagaInstance(GetSagaInstanceRequest) returns (SagaInstanceView);
  rpc RetrySagaStep(RetrySagaStepRequest) returns (RetrySagaStepResponse);
  rpc CompensateSaga(CompensateSagaRequest) returns (CompensateSagaResponse);
}

// operator-service (operator.v1)
service OperatorService {
  rpc RegisterOperator(RegisterOperatorRequest) returns (RegisterOperatorResponse);
  rpc GetOperator(GetOperatorRequest) returns (OperatorView);
  rpc UpdateOperatorStatus(UpdateOperatorStatusRequest) returns (UpdateOperatorStatusResponse);
}
```

---

## 9. Proto Compilation

```toml
# Cargo.toml dependencies for gRPC
[dependencies]
tonic = "0.9"
prost = "0.12"

[build-dependencies]
tonic-build = "0.9"

# build.rs
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos(
        &[
            "proto/common/v1/money.proto",
            "proto/orchestration/v1/orchestration.proto",
            "proto/iam/v1/iam.proto",
            "proto/connector/v1/connector.proto",
            "proto/reconciliation/v1/reconciliation.proto",
            "proto/invoice/v1/invoice.proto",
            "proto/subscription/v1/subscription.proto",
            "proto/compliance/v1/compliance.proto",
            "proto/dispute/v1/dispute.proto",
            "proto/risk/v1/risk.proto",
            "proto/ai_assistant/v1/ai_assistant.proto",
            "proto/document/v1/document.proto",
            "proto/notification/v1/notification.proto",
            "proto/analytics/v1/analytics.proto",
            "proto/saga/v1/saga.proto",
            "proto/operator/v1/operator.proto",
        ],
        &["proto/"],
    )?;
    Ok(())
}
```
