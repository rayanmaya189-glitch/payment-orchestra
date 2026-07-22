//! TDD tests for invoice-service.

#[cfg(test)]
mod tests {
    use uuid::Uuid;
    use chrono::{Utc, Duration};

    use crate::domain::*;
    use crate::commands::*;
    use crate::queries::*;
    use crate::repository::*;

    fn setup_handler() -> (InvoiceCommandHandler<InMemoryInvoiceRepository>, InMemoryInvoiceRepository) {
        let repo = InMemoryInvoiceRepository::new();
        let handler = InvoiceCommandHandler::new(repo.clone());
        (handler, repo)
    }

    fn make_line_items() -> Vec<InvoiceLineItem> {
        vec![
            InvoiceLineItem { description: "Widget".into(), amount_minor: 5000, quantity: 2, unit_price_minor: 2500 },
            InvoiceLineItem { description: "Shipping".into(), amount_minor: 1000, quantity: 1, unit_price_minor: 1000 },
        ]
    }

    #[tokio::test]
    async fn test_create_invoice_success() {
        let (handler, _) = setup_handler();
        let operator_id = Uuid::now_v7();

        let result = handler.create_invoice(CreateInvoice {
            operator_id,
            order_reference: "ORD-001".into(),
            line_items: make_line_items(),
            currency: "AED".into(),
            due_date: Utc::now() + Duration::days(30),
            recipient_email: Some("customer@example.com".into()),
        }).await.unwrap();

        assert_eq!(result.status, InvoiceStatus::Draft);
        assert_eq!(result.total_amount_minor, 6000); // 5000 + 1000
    }

    #[tokio::test]
    async fn test_create_duplicate_order_invoice_rejected() {
        let (handler, _) = setup_handler();
        let operator_id = Uuid::now_v7();

        handler.create_invoice(CreateInvoice {
            operator_id,
            order_reference: "ORD-001".into(),
            line_items: make_line_items(),
            currency: "AED".into(),
            due_date: Utc::now() + Duration::days(30),
            recipient_email: None,
        }).await.unwrap();

        let result = handler.create_invoice(CreateInvoice {
            operator_id,
            order_reference: "ORD-001".into(),
            line_items: make_line_items(),
            currency: "AED".into(),
            due_date: Utc::now() + Duration::days(30),
            recipient_email: None,
        }).await;

        assert!(matches!(result, Err(InvoiceError::DuplicateOrderInvoice(_))));
    }

    #[tokio::test]
    async fn test_create_invoice_empty_line_items_rejected() {
        let (handler, _) = setup_handler();
        let operator_id = Uuid::now_v7();

        let result = handler.create_invoice(CreateInvoice {
            operator_id,
            order_reference: "ORD-002".into(),
            line_items: vec![],
            currency: "AED".into(),
            due_date: Utc::now() + Duration::days(30),
            recipient_email: None,
        }).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_invoice() {
        let (handler, _) = setup_handler();
        let operator_id = Uuid::now_v7();

        let created = handler.create_invoice(CreateInvoice {
            operator_id,
            order_reference: "ORD-003".into(),
            line_items: make_line_items(),
            currency: "AED".into(),
            due_date: Utc::now() + Duration::days(30),
            recipient_email: None,
        }).await.unwrap();

        let result = handler.send_invoice(SendInvoice {
            invoice_id: created.invoice_id,
        }).await.unwrap();

        assert_eq!(result.status, InvoiceStatus::Sent);
    }

    #[tokio::test]
    async fn test_cancel_invoice() {
        let (handler, _) = setup_handler();
        let operator_id = Uuid::now_v7();

        let created = handler.create_invoice(CreateInvoice {
            operator_id,
            order_reference: "ORD-004".into(),
            line_items: make_line_items(),
            currency: "AED".into(),
            due_date: Utc::now() + Duration::days(30),
            recipient_email: None,
        }).await.unwrap();

        let result = handler.cancel_invoice(CancelInvoice {
            invoice_id: created.invoice_id,
            reason: Some("Customer requested cancellation".into()),
        }).await.unwrap();

        assert_eq!(result.status, InvoiceStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_cancel_paid_invoice_rejected() {
        let (handler, _repo) = setup_handler();
        let operator_id = Uuid::now_v7();

        let created = handler.create_invoice(CreateInvoice {
            operator_id,
            order_reference: "ORD-005".into(),
            line_items: make_line_items(),
            currency: "AED".into(),
            due_date: Utc::now() + Duration::days(30),
            recipient_email: None,
        }).await.unwrap();

        // Send it first, then pay in full
        handler.send_invoice(SendInvoice { invoice_id: created.invoice_id }).await.unwrap();
        handler.link_payment(LinkPaymentToInvoice {
            invoice_id: created.invoice_id,
            payment_intent_id: Uuid::now_v7(),
            amount_minor: 6000,
        }).await.unwrap();

        // Try to cancel (should fail because already paid)
        let result = handler.cancel_invoice(CancelInvoice {
            invoice_id: created.invoice_id,
            reason: None,
        }).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invoice_paid_on_full_payment() {
        let (handler, _) = setup_handler();
        let operator_id = Uuid::now_v7();

        let created = handler.create_invoice(CreateInvoice {
            operator_id,
            order_reference: "ORD-006".into(),
            line_items: make_line_items(),
            currency: "AED".into(),
            due_date: Utc::now() + Duration::days(30),
            recipient_email: None,
        }).await.unwrap();

        // Send first
        handler.send_invoice(SendInvoice { invoice_id: created.invoice_id }).await.unwrap();

        // Full payment
        let result = handler.link_payment(LinkPaymentToInvoice {
            invoice_id: created.invoice_id,
            payment_intent_id: Uuid::now_v7(),
            amount_minor: 6000,
        }).await.unwrap();

        assert_eq!(result.status, InvoiceStatus::Paid);
        assert_eq!(result.paid_amount_minor, 6000);
    }

    #[tokio::test]
    async fn test_invoice_partially_paid() {
        let (handler, _) = setup_handler();
        let operator_id = Uuid::now_v7();

        let created = handler.create_invoice(CreateInvoice {
            operator_id,
            order_reference: "ORD-007".into(),
            line_items: make_line_items(),
            currency: "AED".into(),
            due_date: Utc::now() + Duration::days(30),
            recipient_email: None,
        }).await.unwrap();

        handler.send_invoice(SendInvoice { invoice_id: created.invoice_id }).await.unwrap();

        // Partial payment
        let result = handler.link_payment(LinkPaymentToInvoice {
            invoice_id: created.invoice_id,
            payment_intent_id: Uuid::now_v7(),
            amount_minor: 2000,
        }).await.unwrap();

        assert_eq!(result.status, InvoiceStatus::PartiallyPaid);
    }

    #[tokio::test]
    async fn test_mark_invoice_overdue() {
        let (handler, _) = setup_handler();
        let operator_id = Uuid::now_v7();

        let created = handler.create_invoice(CreateInvoice {
            operator_id,
            order_reference: "ORD-008".into(),
            line_items: make_line_items(),
            currency: "AED".into(),
            due_date: Utc::now() - Duration::days(10), // Past due
            recipient_email: None,
        }).await.unwrap();

        handler.send_invoice(SendInvoice { invoice_id: created.invoice_id }).await.unwrap();

        let result = handler.mark_overdue(MarkInvoiceOverdue {
            invoice_id: created.invoice_id,
        }).await.unwrap();

        assert_eq!(result.status, InvoiceStatus::Overdue);
    }

    #[tokio::test]
    async fn test_query_invoice() {
        let repo = InMemoryInvoiceRepository::new();
        let handler = InvoiceCommandHandler::new(repo.clone());
        let query_handler = InvoiceQueryHandler::new(repo.clone());
        let operator_id = Uuid::now_v7();

        let created = handler.create_invoice(CreateInvoice {
            operator_id,
            order_reference: "ORD-QUERY".into(),
            line_items: make_line_items(),
            currency: "AED".into(),
            due_date: Utc::now() + Duration::days(30),
            recipient_email: None,
        }).await.unwrap();

        let loaded = query_handler.get_invoice(GetInvoiceQuery {
            invoice_id: created.invoice_id,
        }).await.unwrap();

        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().order_reference, "ORD-QUERY");
    }

    #[tokio::test]
    async fn test_find_invoice_by_order() {
        let repo = InMemoryInvoiceRepository::new();
        let handler = InvoiceCommandHandler::new(repo.clone());
        let query_handler = InvoiceQueryHandler::new(repo.clone());
        let operator_id = Uuid::now_v7();

        handler.create_invoice(CreateInvoice {
            operator_id,
            order_reference: "ORD-FIND".into(),
            line_items: make_line_items(),
            currency: "AED".into(),
            due_date: Utc::now() + Duration::days(30),
            recipient_email: None,
        }).await.unwrap();

        let found = query_handler.find_by_order(FindInvoiceByOrderQuery {
            operator_id,
            order_reference: "ORD-FIND".into(),
        }).await.unwrap();

        assert!(found.is_some());
    }
}
