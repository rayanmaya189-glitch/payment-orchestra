# 08 — Shared Components

## 1. Status Badge

```swift
struct StatusBadge: View {
    let status: String
    
    private var color: Color {
        switch status.lowercased() {
        case "active", "authorized", "captured": return .green
        case "failed", "failedallroutes": return .red
        case "pending", "authorizing", "capturing": return .blue
        case "expired", "voided": return .gray
        case "refunded", "partiallyrefunded": return .orange
        case "warning", "overdue": return .yellow
        default: return .gray
        }
    }
    
    var body: some View {
        Text(status)
            .font(.caption2)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(color.opacity(0.1))
            .foregroundStyle(color)
            .clipShape(Capsule())
    }
}
```

---

## 2. Money Display

```swift
struct MoneyDisplay: View {
    let amount: Money
    var weight: Font.Weight = .regular
    
    private var formatted: String {
        let formatter = NumberFormatter()
        formatter.numberStyle = .currency
        formatter.currencyCode = amount.currency
        formatter.minimumFractionDigits = precision
        formatter.maximumFractionDigits = precision
        return formatter.string(from: NSNumber(value: Double(amount.amountMinorUnits) / pow(10, Double(precision)))) ?? ""
    }
    
    private var precision: Int {
        switch amount.currency {
        case "BHD", "KWD": return 3
        case "JPY": return 0
        default: return 2
        }
    }
    
    var body: some View {
        Text(formatted)
            .font(.subheadline.monospaced())
            .fontWeight(weight)
    }
}

// Money model
struct Money: Codable, Identifiable, Hashable {
    var id: Int64 { amountMinorUnits }
    let amountMinorUnits: Int64
    let currency: String
    
    static let zero = Money(amountMinorUnits: 0, currency: "AED")
}
```

---

## 3. Loading & Empty States

```swift
struct LoadingView: View {
    var body: some View {
        VStack(spacing: 16) {
            ProgressView()
            Text("Loading...")
                .font(.subheadline)
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

struct EmptyStateView: View {
    let icon: String
    let title: String
    let description: String
    var actionTitle: String? = nil
    var action: (() -> Void)? = nil
    
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: icon)
                .font(.largeTitle)
                .foregroundStyle(.secondary)
            
            Text(title)
                .font(.headline)
            
            Text(description)
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)
            
            if let actionTitle = actionTitle, let action = action {
                Button(actionTitle, action: action)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}
```

---

## 4. Date Formatting

```swift
extension Date {
    var relativeFormatted: String {
        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .abbreviated
        return formatter.localizedString(for: self, relativeTo: Date())
    }
    
    var shortFormatted: String {
        let formatter = DateFormatter()
        formatter.dateStyle = .short
        formatter.timeStyle = .none
        return formatter.string(from: self)
    }
}
```
