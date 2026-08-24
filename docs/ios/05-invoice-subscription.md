# 05 — Invoice & Subscription

## 1. Invoice List

```swift
struct InvoiceListView: View {
    @StateObject var viewModel = Container.invoiceListVM()
    
    var body: some View {
        List {
            ForEach(viewModel.invoices) { invoice in
                NavigationLink(destination: InvoiceDetailView(invoice: invoice)) {
                    InvoiceRow(invoice: invoice)
                }
            }
        }
        .navigationTitle("Invoices")
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                NavigationLink(destination: CreateInvoiceView()) {
                    Image(systemName: "plus")
                }
            }
        }
        .refreshable { await viewModel.refresh() }
    }
}

struct InvoiceRow: View {
    let invoice: Invoice
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                VStack(alignment: .leading) {
                    Text(invoice.orderReference)
                        .font(.headline)
                    Text(invoice.id.prefix(12).appending("..."))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                StatusBadge(status: invoice.status)
            }
            
            HStack {
                VStack(alignment: .leading) {
                    Text("Total")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                    MoneyDisplay(amount: invoice.totalAmount)
                }
                
                Spacer()
                
                VStack(alignment: .leading) {
                    Text("Paid")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                    MoneyDisplay(amount: invoice.paidAmount)
                }
                
                Spacer()
                
                VStack(alignment: .leading) {
                    Text("Due")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                    Text(invoice.dueDate.formatted(date: .abbreviated, time: .omitted))
                        .font(.subheadline)
                }
            }
            
            // Progress bar
            ProgressView(value: Double(invoice.paidAmount.amountMinorUnits), 
                        total: Double(invoice.totalAmount.amountMinorUnits))
        }
        .padding(.vertical, 4)
    }
}
```

---

## 2. Subscription List

```swift
struct SubscriptionListView: View {
    @StateObject var viewModel = Container.subscriptionListVM()
    
    var body: some View {
        List {
            ForEach(viewModel.subscriptions) { subscription in
                SubscriptionRow(
                    subscription: subscription,
                    onPause: { viewModel.pause(subscription) },
                    onResume: { viewModel.resume(subscription) },
                    onCancel: { viewModel.cancel(subscription) }
                )
            }
        }
        .navigationTitle("Subscriptions")
        .refreshable { await viewModel.refresh() }
    }
}

struct SubscriptionRow: View {
    let subscription: Subscription
    let onPause: () -> Void
    let onResume: () -> Void
    let onCancel: () -> Void
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                VStack(alignment: .leading) {
                    Text(subscription.customerName)
                        .font(.headline)
                    Text(subscription.planName)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                StatusBadge(status: subscription.status)
            }
            
            HStack {
                VStack(alignment: .leading) {
                    Text("Amount/Period")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                    MoneyDisplay(amount: subscription.amount)
                }
                
                Spacer()
                
                VStack(alignment: .leading) {
                    Text("Next Billing")
                        .font(.caption2)
                        .foregroundStyle(.secondary)
                    Text(subscription.currentPeriodEnd.formatted(date: .abbreviated, time: .omitted))
                        .font(.subheadline)
                }
            }
            
            HStack(spacing: 8) {
                switch subscription.status {
                case "active":
                    Button("Pause", action: onPause).buttonStyle(.bordered)
                    Button("Cancel", role: .destructive, action: onCancel).buttonStyle(.bordered)
                case "paused":
                    Button("Resume", action: onResume).buttonStyle(.borderedProminent)
                default:
                    EmptyView()
                }
            }
        }
        .padding(.vertical, 4)
    }
}
```
