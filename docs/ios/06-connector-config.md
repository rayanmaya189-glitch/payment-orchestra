# 06 — Connector Configuration

## 1. Gateway Profile Detail

```swift
struct GatewayProfileDetailView: View {
    let profile: GatewayProfile
    
    var body: some View {
        List {
            Section("Transaction Limits") {
                LabeledContent("Min Amount", value: profile.minAmount.formatted())
                LabeledContent("Max Amount", value: profile.maxAmount.formatted())
                LabeledContent("Daily Volume", value: profile.dailyVolumeLimit.formatted())
                LabeledContent("Monthly Volume", value: profile.monthlyVolumeLimit.formatted())
            }
            
            Section("Fee Structure") {
                LabeledContent("Fixed Fee", value: profile.fixedFee.formatted())
                LabeledContent("Percentage", value: "\(profile.percentageFeeBps / 100)%")
                LabeledContent("Cross-Border", value: "\(profile.crossBorderFeeBps / 100)%")
            }
            
            Section("Volume Usage") {
                VStack(spacing: 12) {
                    VStack(alignment: .leading) {
                        Text("Daily")
                            .font(.caption)
                        ProgressView(value: Double(profile.dailyVolumeUsed.amountMinorUnits), 
                                    total: Double(profile.dailyVolumeLimit.amountMinorUnits))
                        Text("\(profile.dailyVolumeUsed.formatted()) / \(profile.dailyVolumeLimit.formatted())")
                            .font(.caption2)
                    }
                    
                    VStack(alignment: .leading) {
                        Text("Monthly")
                            .font(.caption)
                        ProgressView(value: Double(profile.monthlyVolumeUsed.amountMinorUnits), 
                                    total: Double(profile.monthlyVolumeLimit.amountMinorUnits))
                        Text("\(profile.monthlyVolumeUsed.formatted()) / \(profile.monthlyVolumeLimit.formatted())")
                            .font(.caption2)
                    }
                }
            }
            
            Section("Rate Limits") {
                LabeledContent("Per Second", value: "\(profile.rateLimitPerSecond)")
                LabeledContent("Per Day", value: "\(profile.rateLimitPerDay)")
            }
            
            Section("Monitoring") {
                LabeledContent("Success Rate Alert", value: "\(profile.successRateThreshold * 100, specifier: "%.1f")%")
                LabeledContent("Latency Alert", value: "\(profile.latencyThresholdMs)ms")
                Toggle("Auto-Disable", isOn: .constant(profile.autoDisableOnLowSuccess))
                    .disabled(true)
            }
        }
        .navigationTitle(profile.connectorName)
    }
}
```

---

## 2. Gateway Rotation Strategy

```swift
struct RotationStrategyPicker: View {
    @Binding var selected: RotationStrategy
    
    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Rotation Strategy")
                .font(.headline)
            
            ForEach(RotationStrategy.allCases) { strategy in
                Button(action: { selected = strategy }) {
                    HStack {
                        Image(systemName: selected == strategy ? "checkmark.circle.fill" : "circle")
                            .foregroundStyle(selected == strategy ? .blue : .secondary)
                        
                        VStack(alignment: .leading) {
                            Text(strategy.displayName)
                                .font(.body)
                            Text(strategy.description)
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                        
                        Spacer()
                    }
                    .padding(.vertical, 4)
                }
                .buttonStyle(.plain)
            }
        }
    }
}

enum RotationStrategy: String, CaseIterable, Identifiable {
    case priority
    case roundRobin
    case weightedRoundRobin
    case costBased
    case successRateBased
    case volumeCapped
    
    var id: String { rawValue }
    
    var displayName: String {
        switch self {
        case .priority: return "Priority"
        case .roundRobin: return "Round Robin"
        case .weightedRoundRobin: return "Weighted Round Robin"
        case .costBased: return "Cost Based"
        case .successRateBased: return "Success Rate"
        case .volumeCapped: return "Volume Capped"
        }
    }
    
    var description: String {
        switch self {
        case .priority: return "Fixed order — always try gateway 1 first"
        case .roundRobin: return "Distribute evenly across gateways"
        case .weightedRoundRobin: return "Distribute by weight"
        case .costBased: return "Select cheapest gateway per transaction"
        case .successRateBased: return "Select highest success rate gateway"
        case .volumeCapped: return "Rotate until one hits daily limit"
        }
    }
}
```
