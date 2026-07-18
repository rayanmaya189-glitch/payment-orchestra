# 01 — Dashboard

## 1. Main Dashboard

```swift
struct DashboardView: View {
    @StateObject var viewModel = Container.dashboardVM()
    
    var body: some View {
        ScrollView {
            VStack(spacing: 16) {
                // Stats grid
                LazyVGrid(columns: Array(repeating: GridItem(.flexible()), count: 2), spacing: 12) {
                    ForEach(viewModel.stats) { stat in
                        StatsCard(stat: stat)
                            .onTapGesture { navigateToDetail(stat) }
                    }
                }
                
                // Charts
                AuthRateChart(data: viewModel.hourlyRates)
                DeclineReasonPieChart(data: viewModel.declineBreakdown)
                
                // Recent activity
                RecentActivityList(
                    transactions: viewModel.recentTransactions,
                    onSelect: { navigateToPaymentDetail($0) }
                )
            }
            .padding()
        }
        .refreshable {
            await viewModel.refresh()
        }
        .navigationTitle("Dashboard")
    }
}

struct StatsCard: View {
    let stat: DashboardStat
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Image(systemName: stat.icon)
                    .foregroundStyle(.blue)
                Spacer()
                if let trend = stat.trend {
                    TrendBadge(trend: trend)
                }
            }
            
            Text(stat.value)
                .font(.title2.bold())
            
            Text(stat.title)
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding()
        .background(.background)
        .clipShape(RoundedRectangle(cornerRadius: 12))
        .shadow(color: .black.opacity(0.05), radius: 4, y: 2)
    }
}

struct TrendBadge: View {
    let trend: Float
    
    var body: some View {
        HStack(spacing: 2) {
            Image(systemName: trend > 0 ? "arrow.up" : "arrow.down")
                .font(.caption2)
            Text("\(abs(trend), specifier: "%.1f")%")
                .font(.caption2)
        }
        .foregroundStyle(trend > 0 ? .green : trend < 0 ? .red : .secondary)
    }
}
```

---

## 2. Charts (Swift Charts)

```swift
import Charts

struct AuthRateChart: View {
    let data: [HourlyRate]
    
    var body: some View {
        VStack(alignment: .leading) {
            Text("Authorization Rate")
                .font(.headline)
            
            Chart(data) { item in
                LineMark(
                    x: .value("Hour", item.hour),
                    y: .value("Rate", item.rate)
                )
                .foregroundStyle(.blue)
                .interpolationMethod(.catmullRom)
            }
            .chartYAxis {
                AxisMarks(position: .leading)
            }
            .frame(height: 200)
        }
        .padding()
        .background(.background)
        .clipShape(RoundedRectangle(cornerRadius: 12))
    }
}

struct DeclineReasonPieChart: View {
    let data: [DeclineReasonCount]
    
    var body: some View {
        VStack(alignment: .leading) {
            Text("Decline Reasons")
                .font(.headline)
            
            Chart(data) { item in
                SectorMark(
                    angle: .value("Count", item.count),
                    innerRadius: .ratio(0.5)
                )
                .foregroundStyle(item.color)
                .annotation(position: .overlay) {
                    Text("\(item.count)")
                        .font(.caption2)
                        .foregroundStyle(.white)
                }
            }
            .frame(height: 200)
            
            // Legend
            ForEach(data) { item in
                HStack {
                    Circle()
                        .fill(item.color)
                        .frame(width: 10, height: 10)
                    Text(item.reason)
                        .font(.caption)
                    Spacer()
                    Text("\(item.count)")
                        .font(.caption)
                }
            }
        }
        .padding()
        .background(.background)
        .clipShape(RoundedRectangle(cornerRadius: 12))
    }
}
```

---

## 3. Recent Activity List

```swift
struct RecentActivityList: View {
    let transactions: [PaymentIntent]
    let onSelect: (PaymentIntent) -> Void
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Recent Activity")
                .font(.headline)
            
            ForEach(transactions.prefix(10)) { transaction in
                PaymentRow(transaction: transaction)
                    .onTapGesture { onSelect(transaction) }
                
                if transaction.id != transactions.prefix(10).last?.id {
                    Divider()
                }
            }
        }
        .padding()
        .background(.background)
        .clipShape(RoundedRectangle(cornerRadius: 12))
    }
}

struct PaymentRow: View {
    let transaction: PaymentIntent
    
    var body: some View {
        HStack {
            VStack(alignment: .leading) {
                Text(transaction.id.prefix(12).appending("..."))
                    .font(.subheadline.monospaced())
                Text(transaction.createdAt.relativeFormatted)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            
            Spacer()
            
            VStack(alignment: .trailing) {
                MoneyDisplay(amount: transaction.amount, currency: transaction.currency)
                StatusBadge(status: transaction.status)
            }
        }
        .contentShape(Rectangle())
    }
}
```
