# 06 — Connector Configuration

## 1. Connector List

```kotlin
@Composable
fun ConnectorListScreen(viewModel: ConnectorViewModel = hiltViewModel()) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.connectors)) },
                actions = {
                    IconButton(onClick = { navigateToConnectNew() }) {
                        Icon(Icons.Default.Add, contentDescription = "Connect New")
                    }
                }
            )
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier.padding(padding),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            items(uiState.connectors) { connector ->
                ConnectorCard(connector = connector)
            }
        }
    }
}

@Composable
fun ConnectorCard(connector: Connector) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .clickable { navigateToConnectorDetail(connector.id) }
    ) {
        Row(
            modifier = Modifier.padding(16.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Connector icon/logo
            AsyncImage(
                model = connector.logoUrl,
                contentDescription = connector.name,
                modifier = Modifier.size(48.dp)
            )
            
            Spacer(modifier = Modifier.width(12.dp))
            
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = connector.name,
                    style = MaterialTheme.typography.titleSmall
                )
                Text(
                    text = connector.status.displayName,
                    style = MaterialTheme.typography.bodySmall,
                    color = connector.status.color
                )
            }
            
            Column(horizontalAlignment = Alignment.End) {
                Row {
                    connector.supportedCardSchemes.forEach { scheme ->
                        CardSchemeIcon(scheme = scheme, modifier = Modifier.size(20.dp))
                    }
                }
                Text(
                    text = connector.settlementFormat,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}
```

---

## 2. Gateway Profile Detail

```kotlin
@Composable
fun GatewayProfileDetail(profile: GatewayProfile) {
    Column(
        modifier = Modifier.padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        // Transaction Limits
        ProfileSection(title = stringResource(R.string.transaction_limits)) {
            LimitRow(label = stringResource(R.string.min_amount), value = profile.minAmount.formatMoney())
            LimitRow(label = stringResource(R.string.max_amount), value = profile.maxAmount.formatMoney())
            LimitRow(label = stringResource(R.string.daily_volume_limit), value = profile.dailyVolumeLimit.formatMoney())
            LimitRow(label = stringResource(R.string.monthly_volume_limit), value = profile.monthlyVolumeLimit.formatMoney())
        }
        
        // Fee Structure
        ProfileSection(title = stringResource(R.string.fee_structure)) {
            FeeRow(label = stringResource(R.string.fixed_fee), value = profile.fixedFee.formatMoney())
            FeeRow(label = stringResource(R.string.percentage_fee), value = "${profile.percentageFeeBps / 100}%")
            FeeRow(label = stringResource(R.string.cross_border_fee), value = "${profile.crossBorderFeeBps / 100}%")
        }
        
        // Volume Usage
        ProfileSection(title = stringResource(R.string.volume_usage)) {
            VolumeUsageBar(
                dailyUsed = profile.dailyVolumeUsed,
                dailyLimit = profile.dailyVolumeLimit,
                monthlyUsed = profile.monthlyVolumeUsed,
                monthlyLimit = profile.monthlyVolumeLimit
            )
        }
        
        // Rate Limits
        ProfileSection(title = stringResource(R.string.rate_limits)) {
            LimitRow(label = stringResource(R.string.per_second), value = profile.rateLimitPerSecond.toString())
            LimitRow(label = stringResource(R.string.per_day), value = profile.rateLimitPerDay.toString())
        }
    }
}

@Composable
fun ProfileSection(title: String, content: @Composable ColumnScope.() -> Unit) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(
                text = title,
                style = MaterialTheme.typography.titleMedium
            )
            Spacer(modifier = Modifier.height(12.dp))
            content()
        }
    }
}

@Composable
fun VolumeUsageBar(dailyUsed: Money, dailyLimit: Money, monthlyUsed: Money, monthlyLimit: Money) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Column {
            Text(
                text = stringResource(R.string.daily_volume),
                style = MaterialTheme.typography.labelSmall
            )
            LinearProgressIndicator(
                progress = { dailyUsed.amountMinorUnits.toFloat() / dailyLimit.amountMinorUnits.toFloat() },
                modifier = Modifier.fillMaxWidth()
            )
            Text(
                text = "${dailyUsed.formatMoney()} / ${dailyLimit.formatMoney()}",
                style = MaterialTheme.typography.bodySmall
            )
        }
        
        Column {
            Text(
                text = stringResource(R.string.monthly_volume),
                style = MaterialTheme.typography.labelSmall
            )
            LinearProgressIndicator(
                progress = { monthlyUsed.amountMinorUnits.toFloat() / monthlyLimit.amountMinorUnits.toFloat() },
                modifier = Modifier.fillMaxWidth()
            )
            Text(
                text = "${monthlyUsed.formatMoney()} / ${monthlyLimit.formatMoney()}",
                style = MaterialTheme.typography.bodySmall
            )
        }
    }
}
```

---

## 3. Gateway Rotation Strategy Selector

```kotlin
@Composable
fun RotationStrategySelector(
    currentStrategy: RotationStrategy,
    onChange: (RotationStrategy) -> Unit
) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(
                text = stringResource(R.string.rotation_strategy),
                style = MaterialTheme.typography.titleMedium
            )
            
            Spacer(modifier = Modifier.height(12.dp))
            
            RotationStrategy.values().forEach { strategy ->
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clickable { onChange(strategy) }
                        .padding(vertical = 8.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    RadioButton(
                        selected = currentStrategy == strategy,
                        onClick = { onChange(strategy) }
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                    Column {
                        Text(
                            text = strategy.displayName,
                            style = MaterialTheme.typography.bodyMedium
                        )
                        Text(
                            text = strategy.description,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }
            }
        }
    }
}

enum class RotationStrategy(val displayName: String, val description: String) {
    PRIORITY("Priority", "Fixed order — always try gateway 1 first"),
    ROUND_ROBIN("Round Robin", "Distribute evenly across gateways"),
    WEIGHTED_ROUND_ROBIN("Weighted Round Robin", "Distribute by weight"),
    COST_BASED("Cost Based", "Select cheapest gateway per transaction"),
    SUCCESS_RATE("Success Rate", "Select highest success rate gateway"),
    VOLUME_CAPPED("Volume Capped", "Rotate until one hits daily limit")
}
```
