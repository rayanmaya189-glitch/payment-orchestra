# 03 — Reconciliation

## 1. Reconciliation Dashboard

```swift
struct ReconciliationView: View {
    @StateObject var viewModel = Container.reconciliationVM()
    
    var body: some View {
        List {
            // Stats
            Section("Overview") {
                LabeledContent("Matched") {
                    Text("\(viewModel.stats.matched)")
                }
                LabeledContent("Unmatched") {
                    Text("\(viewModel.stats.unmatched)")
                        .foregroundStyle(viewModel.stats.unmatched > 0 ? .red : .primary)
                }
                LabeledContent("Match Rate") {
                    Text("\(viewModel.stats.matchRate, specifier: "%.1f")%")
                }
            }
            
            // Quick actions
            Section("Actions") {
                NavigationLink("Exception Queue", destination: ExceptionQueueView())
                NavigationLink("Settlement Batches", destination: SettlementBatchesView())
                NavigationLink("Ledger View", destination: LedgerView())
            }
            
            // Match rate chart
            Section("Match Rate Trend") {
                MatchRateChart(data: viewModel.matchRateTrend)
                    .frame(height: 200)
            }
        }
        .navigationTitle("Reconciliation")
        .refreshable { await viewModel.refresh() }
    }
}
```

---

## 2. Exception Queue

```swift
struct ExceptionQueueView: View {
    @StateObject var viewModel = Container.exceptionQueueVM()
    
    var body: some View {
        List {
            ForEach(viewModel.exceptions) { exception in
                ExceptionCard(
                    exception: exception,
                    onMatch: { viewModel.match(exception) },
                    onFlag: { viewModel.flag(exception) },
                    onAiSuggest: { viewModel.aiSuggest(exception) }
                )
            }
        }
        .navigationTitle("Exceptions")
        .refreshable { await viewModel.refresh() }
    }
}

struct ExceptionCard: View {
    let exception: ReconciliationException
    let onMatch: () -> Void
    let onFlag: () -> Void
    let onAiSuggest: () -> Void
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(exception.classification.displayName)
                    .font(.caption)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(exception.classification.color.opacity(0.1))
                    .foregroundStyle(exception.classification.color)
                    .clipShape(Capsule())
                
                Spacer()
                
                StatusBadge(status: exception.status)
            }
            
            Text(exception.acquirerReference)
                .font(.headline)
            
            MoneyDisplay(amount: exception.amount)
            
            Text(exception.detectedAt.relativeFormatted)
                .font(.caption)
                .foregroundStyle(.secondary)
            
            HStack(spacing: 8) {
                Button("Match", action: onMatch)
                    .buttonStyle(.bordered)
                
                Button("Flag", action: onFlag)
                    .buttonStyle(.bordered)
                
                Button {
                    onAiSuggest()
                } label: {
                    Label("AI Suggest", systemImage: "sparkles")
                }
                .buttonStyle(.borderedProminent)
            }
        }
        .padding()
        .background(.background)
        .clipShape(RoundedRectangle(cornerRadius: 12))
    }
}
```
