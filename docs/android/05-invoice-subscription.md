# 05 — Invoice & Subscription

## 1. Invoice List

```kotlin
@Composable
fun InvoiceListScreen(viewModel: InvoiceViewModel = hiltViewModel()) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.invoices)) },
                actions = {
                    IconButton(onClick = { navigateToCreateInvoice() }) {
                        Icon(Icons.Default.Add, contentDescription = "Create Invoice")
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
            items(uiState.invoices) { invoice ->
                InvoiceCard(invoice = invoice)
            }
        }
    }
}

@Composable
fun InvoiceCard(invoice: Invoice) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Column {
                    Text(
                        text = invoice.orderReference,
                        style = MaterialTheme.typography.titleSmall
                    )
                    Text(
                        text = invoice.id.take(12) + "...",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
                StatusBadge(status = invoice.status)
            }
            
            Spacer(modifier = Modifier.height(8.dp))
            
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Column {
                    Text(
                        text = stringResource(R.string.total),
                        style = MaterialTheme.typography.labelSmall
                    )
                    MoneyDisplay(amount = invoice.totalAmount)
                }
                
                Column {
                    Text(
                        text = stringResource(R.string.paid),
                        style = MaterialTheme.typography.labelSmall
                    )
                    MoneyDisplay(amount = invoice.paidAmount)
                }
                
                Column {
                    Text(
                        text = stringResource(R.string.due_date),
                        style = MaterialTheme.typography.labelSmall
                    )
                    Text(
                        text = invoice.dueDate.formatShort(),
                        style = MaterialTheme.typography.bodyMedium
                    )
                }
            }
            
            // Payment progress
            if (invoice.totalAmount.amountMinorUnits > 0) {
                Spacer(modifier = Modifier.height(8.dp))
                LinearProgressIndicator(
                    progress = { invoice.paidAmount.amountMinorUnits.toFloat() / invoice.totalAmount.amountMinorUnits.toFloat() },
                    modifier = Modifier.fillMaxWidth()
                )
            }
        }
    }
}
```

---

## 2. Subscription List

```kotlin
@Composable
fun SubscriptionListScreen(viewModel: SubscriptionViewModel = hiltViewModel()) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()

    LazyColumn(
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        items(uiState.subscriptions) { subscription ->
            SubscriptionCard(
                subscription = subscription,
                onPause = { viewModel.pauseSubscription(subscription.id) },
                onResume = { viewModel.resumeSubscription(subscription.id) },
                onCancel = { viewModel.cancelSubscription(subscription.id) }
            )
        }
    }
}

@Composable
fun SubscriptionCard(
    subscription: Subscription,
    onPause: () -> Unit,
    onResume: () -> Unit,
    onCancel: () -> Unit
) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Column {
                    Text(
                        text = subscription.customerName,
                        style = MaterialTheme.typography.titleSmall
                    )
                    Text(
                        text = subscription.planName,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
                StatusBadge(status = subscription.status)
            }
            
            Spacer(modifier = Modifier.height(8.dp))
            
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Column {
                    Text(
                        text = stringResource(R.string.amount_per_period),
                        style = MaterialTheme.typography.labelSmall
                    )
                    MoneyDisplay(amount = subscription.amount)
                }
                
                Column {
                    Text(
                        text = stringResource(R.string.next_billing),
                        style = MaterialTheme.typography.labelSmall
                    )
                    Text(
                        text = subscription.currentPeriodEnd.formatShort(),
                        style = MaterialTheme.typography.bodyMedium
                    )
                }
            }
            
            Spacer(modifier = Modifier.height(12.dp))
            
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                when (subscription.status) {
                    SubscriptionStatus.ACTIVE -> {
                        OutlinedButton(onClick = onPause, modifier = Modifier.weight(1f)) {
                            Text(stringResource(R.string.pause))
                        }
                        OutlinedButton(
                            onClick = onCancel,
                            modifier = Modifier.weight(1f),
                            colors = ButtonDefaults.outlinedButtonColors(
                                contentColor = MaterialTheme.colorScheme.error
                            )
                        ) {
                            Text(stringResource(R.string.cancel))
                        }
                    }
                    SubscriptionStatus.PAUSED -> {
                        FilledButton(onClick = onResume, modifier = Modifier.weight(1f)) {
                            Text(stringResource(R.string.resume))
                        }
                    }
                    else -> { /* No actions for other states */ }
                }
            }
        }
    }
}
```
