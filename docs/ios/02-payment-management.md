# 02 — Payment Management

## 1. Payment List

```swift
struct PaymentListView: View {
    @StateObject var viewModel = Container.paymentListVM()
    @State private var showFilter = false
    
    var body: some View {
        List {
            ForEach(viewModel.payments) { payment in
                NavigationLink(destination: PaymentDetailView(paymentId: payment.id)) {
                    PaymentListRow(payment: payment)
                }
            }
            
            if viewModel.hasMore {
                Button("Load More") {
                    viewModel.loadMore()
                }
                .frame(maxWidth: .infinity)
            }
        }
        .navigationTitle("Payments")
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button { showFilter = true } label: {
                    Image(systemName: "line.3.horizontal.decrease.circle")
                }
            }
            ToolbarItem(placement: .topBarTrailing) {
                NavigationLink(destination: CreatePaymentView()) {
                    Image(systemName: "plus")
                }
            }
        }
        .sheet(isPresented: $showFilter) {
            FilterSheet(filters: viewModel.filters) { viewModel.applyFilters($0) }
        }
        .refreshable { await viewModel.refresh() }
    }
}

struct PaymentListRow: View {
    let payment: PaymentIntent
    
    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                Text(payment.id.prefix(12).appending("..."))
                    .font(.subheadline.monospaced())
                Text(payment.createdAt.relativeFormatted)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            
            Spacer()
            
            VStack(alignment: .trailing, spacing: 4) {
                MoneyDisplay(amount: payment.amount, currency: payment.currency)
                StatusBadge(status: payment.status)
            }
        }
    }
}
```

---

## 2. Payment Detail

```swift
struct PaymentDetailView: View {
    @StateObject var viewModel: PaymentDetailViewModel
    
    init(paymentId: String) {
        _viewModel = StateObject(wrappedValue: Container.paymentDetailVM(id: paymentId))
    }
    
    var body: some View {
        List {
            // Header section
            Section("Payment Details") {
                LabeledContent("ID", value: viewModel.payment?.id ?? "")
                LabeledContent("Status") {
                    StatusBadge(status: viewModel.payment?.status ?? "")
                }
                LabeledContent("Created") {
                    Text(viewModel.payment?.createdAt.formatted() ?? "")
                }
            }
            
            // Amount summary
            Section("Amount") {
                LabeledContent("Requested") {
                    MoneyDisplay(amount: viewModel.payment?.requestedAmount ?? .zero)
                }
                LabeledContent("Authorized") {
                    MoneyDisplay(amount: viewModel.payment?.authorizedAmount ?? .zero)
                }
                LabeledContent("Captured") {
                    MoneyDisplay(amount: viewModel.payment?.capturedAmount ?? .zero)
                }
                LabeledContent("Refunded") {
                    MoneyDisplay(amount: viewModel.payment?.refundedAmount ?? .zero)
                }
            }
            
            // Gateway profile
            if let gatewayProfile = viewModel.payment?.gatewayProfile {
                Section("Payment Gateway") {
                    GatewayProfileRow(profile: gatewayProfile)
                    RotationInfoRow(strategy: viewModel.payment?.gatewayRotationStrategy ?? "")
                    
                    if let fees = viewModel.payment?.fees {
                        FeeBreakdownView(fees: fees, amount: viewModel.payment?.amount ?? .zero)
                    }
                }
            }
            
            // Routing timeline
            if let attempts = viewModel.payment?.attempts, !attempts.isEmpty {
                Section("Routing Timeline") {
                    ForEach(attempts) { attempt in
                        RoutingAttemptRow(attempt: attempt)
                    }
                }
            }
            
            // Actions
            Section("Actions") {
                if viewModel.payment?.status == "Authorized" {
                    Button("Capture") { viewModel.capture() }
                    Button("Void", role: .destructive) { viewModel.void() }
                }
                if viewModel.payment?.status == "Captured" {
                    Button("Refund") { viewModel.showRefundSheet() }
                }
            }
        }
        .navigationTitle("Payment Detail")
    }
}
```

### Gateway Profile Section

```swift
struct GatewayProfileRow: View {
    let profile: GatewayProfile
    
    var body: some View {
        NavigationLink(destination: ConnectorDetailView(profileId: profile.id)) {
            HStack {
                Image(systemName: "building.columns")
                    .foregroundStyle(.blue)
                VStack(alignment: .leading) {
                    Text(profile.connectorName)
                        .font(.headline)
                    Text(profile.status)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                Spacer()
                Image(systemName: "chevron.right")
                    .foregroundStyle(.secondary)
            }
        }
    }
}

struct FeeBreakdownView: View {
    let fees: FeeBreakdown
    let amount: Money
    
    var body: some View {
        VStack(spacing: 8) {
            FeeRow(label: "Base Amount", amount: amount)
            FeeRow(label: "Fixed Fee", amount: fees.fixedFee)
            FeeRow(label: "Percentage Fee", amount: fees.percentageFee)
            if fees.crossBorderFee.amountMinorUnits > 0 {
                FeeRow(label: "Cross-Border Fee", amount: fees.crossBorderFee)
            }
            Divider()
            FeeRow(label: "Total Fee", amount: fees.totalFee, weight: .bold)
            FeeRow(label: "Net Amount", 
                   amount: Money(amountMinorUnits: amount.amountMinorUnits - fees.totalFee.amountMinorUnits, currency: amount.currency),
                   weight: .bold)
        }
    }
}

struct FeeRow: View {
    let label: String
    let amount: Money
    var weight: Font.Weight = .regular
    
    var body: some View {
        HStack {
            Text(label)
                .font(.subheadline)
            Spacer()
            MoneyDisplay(amount: amount, weight: weight)
        }
    }
}
```

---

## 3. Create Payment

```swift
struct CreatePaymentView: View {
    @StateObject var viewModel = Container.createPaymentVM()
    
    var body: some View {
        Form {
            Section("Amount") {
                TextField("Amount", text: $viewModel.amount)
                    .keyboardType(.decimalPad)
                
                Picker("Currency", selection: $viewModel.currency) {
                    Text("AED").tag("AED")
                    Text("USD").tag("USD")
                    Text("EUR").tag("EUR")
                    Text("SAR").tag("SAR")
                }
            }
            
            Section("Gateway") {
                Picker("Select Gateway", selection: $viewModel.selectedGateway) {
                    Text("Auto-rotate").tag(nil as GatewayProfile?)
                    ForEach(viewModel.availableGateways) { gateway in
                        Text(gateway.connectorName).tag(gateway as GatewayProfile?)
                    }
                }
            }
            
            Section("Purpose") {
                Picker("Purpose", selection: $viewModel.purpose) {
                    Text("Payment").tag(PaymentPurpose.payment)
                    Text("Card Verification").tag(PaymentPurpose.cardVerification)
                }
                .pickerStyle(.segmented)
            }
            
            Section {
                Button(action: viewModel.createPayment) {
                    if viewModel.isLoading {
                        ProgressView()
                    } else {
                        Text("Create Payment")
                            .frame(maxWidth: .infinity)
                    }
                }
                .disabled(!viewModel.isValid || viewModel.isLoading)
            }
        }
        .navigationTitle("Create Payment")
    }
}
```
