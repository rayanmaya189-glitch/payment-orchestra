# 03 — Reconciliation

## 1. Reconciliation Dashboard

```kotlin
@Composable
fun ReconciliationScreen(viewModel: ReconciliationViewModel = hiltViewModel()) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()

    Scaffold(
        topBar = { TopAppBar(title = { Text(stringResource(R.string.reconciliation)) }) }
    ) { padding ->
        LazyColumn(
            modifier = Modifier.padding(padding),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            // Stats cards
            item {
                ReconciliationStats(stats = uiState.stats)
            }
            
            // Match rate trend
            item {
                MatchRateChart(data = uiState.matchRateTrend)
            }
            
            // Settlement by acquirer
            item {
                SettlementByAcquirerChart(data = uiState.settlementByAcquirer)
            }
            
            // Quick actions
            item {
                QuickActions(
                    onViewExceptions = { navigateToExceptions() },
                    onViewLedger = { navigateToLedger() }
                )
            }
        }
    }
}
```

---

## 2. Exception Queue

```kotlin
@Composable
fun ExceptionQueueScreen(viewModel: ExceptionViewModel = hiltViewModel()) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()

    LazyColumn(
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        items(uiState.exceptions) { exception ->
            ExceptionCard(
                exception = exception,
                onMatch = { viewModel.matchException(exception.id) },
                onFlag = { viewModel.flagException(exception.id) },
                onAiSuggest = { viewModel.aiSuggest(exception.id) }
            )
        }
    }
}

@Composable
fun ExceptionCard(
    exception: ReconciliationException,
    onMatch: () -> Unit,
    onFlag: () -> Unit,
    onAiSuggest: () -> Unit
) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = exception.classification.displayName,
                    style = MaterialTheme.typography.labelMedium,
                    color = exception.classification.color
                )
                StatusBadge(status = exception.status)
            }
            
            Spacer(modifier = Modifier.height(8.dp))
            
            Text(
                text = exception.acquirerReference,
                style = MaterialTheme.typography.bodyLarge
            )
            
            MoneyDisplay(
                amount = exception.amount,
                style = MaterialTheme.typography.bodyMedium
            )
            
            Text(
                text = exception.detectedAt.formatRelative(),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            
            Spacer(modifier = Modifier.height(12.dp))
            
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                OutlinedButton(
                    onClick = onMatch,
                    modifier = Modifier.weight(1f)
                ) {
                    Text(stringResource(R.string.match))
                }
                
                OutlinedButton(
                    onClick = onFlag,
                    modifier = Modifier.weight(1f)
                ) {
                    Text(stringResource(R.string.flag))
                }
                
                FilledTonalButton(
                    onClick = onAiSuggest,
                    modifier = Modifier.weight(1f)
                ) {
                    Icon(
                        imageVector = Icons.Default.AutoAwesome,
                        contentDescription = null,
                        modifier = Modifier.size(16.dp)
                    )
                    Spacer(modifier = Modifier.width(4.dp))
                    Text(stringResource(R.string.ai_suggest))
                }
            }
        }
    }
}
```

---

## 3. Settlement Batch View

```kotlin
@Composable
fun SettlementBatchCard(batch: SettlementBatch) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text(
                    text = batch.id.take(12) + "...",
                    style = MaterialTheme.typography.titleSmall,
                    fontFamily = FontFamily.Monospace
                )
                StatusBadge(status = batch.status)
            }
            
            Text(
                text = batch.connectorName,
                style = MaterialTheme.typography.bodyMedium
            )
            
            Spacer(modifier = Modifier.height(8.dp))
            
            // Progress bar
            LinearProgressIndicator(
                progress = { batch.matched.toFloat() / batch.total.toFloat() },
                modifier = Modifier.fillMaxWidth()
            )
            
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text(
                    text = "${batch.matched}/${batch.total} matched",
                    style = MaterialTheme.typography.bodySmall
                )
                Text(
                    text = batch.ingestedAt.formatRelative(),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }
        }
    }
}
```
