# 02 — Payment Management

## 1. Payment List Screen

```kotlin
@Composable
fun PaymentListScreen(viewModel: PaymentViewModel = hiltViewModel()) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()
    var showFilterDialog by remember { mutableStateOf(false) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.payments)) },
                actions = {
                    IconButton(onClick = { showFilterDialog = true }) {
                        Icon(Icons.Default.FilterList, contentDescription = "Filter")
                    }
                    IconButton(onClick = { navigateToCreatePayment() }) {
                        Icon(Icons.Default.Add, contentDescription = "Create Payment")
                    }
                }
            )
        }
    ) { padding ->
        LazyColumn(
            modifier = Modifier.padding(padding),
            contentPadding = PaddingValues(16.dp)
        ) {
            items(uiState.payments) { payment ->
                PaymentListRow(
                    payment = payment,
                    onClick = { navigateToPaymentDetail(payment.id) }
                )
            }
            
            if (uiState.hasMore) {
                item {
                    Button(
                        onClick = { viewModel.loadMore() },
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Text(stringResource(R.string.load_more))
                    }
                }
            }
        }
    }

    if (showFilterDialog) {
        FilterDialog(
            filters = uiState.filters,
            onApply = { filters ->
                viewModel.applyFilters(filters)
                showFilterDialog = false
            },
            onDismiss = { showFilterDialog = false }
        )
    }
}
```

### Filter Dialog

```kotlin
@Composable
fun FilterDialog(
    filters: PaymentFilters,
    onApply: (PaymentFilters) -> Unit,
    onDismiss: () -> Unit
) {
    var selectedStatus by remember { mutableStateOf(filters.status) }
    var dateRange by remember { mutableStateOf(filters.dateRange) }
    var selectedAcquirer by remember { mutableStateOf(filters.acquirer) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(stringResource(R.string.filter_payments)) },
        text = {
            Column {
                // Status filter
                Text(stringResource(R.string.status))
                LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    items(PaymentStatus.values()) { status ->
                        FilterChip(
                            selected = selectedStatus.contains(status),
                            onClick = {
                                selectedStatus = if (selectedStatus.contains(status)) {
                                    selectedStatus - status
                                } else {
                                    selectedStatus + status
                                }
                            },
                            label = { Text(status.displayName) }
                        )
                    }
                }

                // Date range
                Text(stringResource(R.string.date_range))
                DateRangePicker(
                    startDate = dateRange?.start,
                    endDate = dateRange?.end,
                    onRangeSelected = { start, end ->
                        dateRange = DateRange(start, end)
                    }
                )

                // Acquirer filter
                Text(stringResource(R.string.acquirer))
                // ... acquirer selection
            }
        },
        confirmButton = {
            TextButton(onClick = { onApply(PaymentFilters(selectedStatus, dateRange, selectedAcquirer)) }) {
                Text(stringResource(R.string.apply))
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text(stringResource(R.string.cancel))
            }
        }
    )
}
```

---

## 2. Payment Detail Screen

```kotlin
@Composable
fun PaymentDetailScreen(
    paymentId: String,
    viewModel: PaymentDetailViewModel = hiltViewModel()
) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.payment_detail)) },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { padding ->
        uiState.payment?.let { payment ->
            LazyColumn(
                modifier = Modifier.padding(padding),
                contentPadding = PaddingValues(16.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp)
            ) {
                // Header
                item {
                    PaymentHeader(payment = payment)
                }
                
                // Amount summary
                item {
                    AmountSummary(payment = payment)
                }
                
                // Gateway profile (linked to order)
                item {
                    GatewayProfileSection(payment = payment)
                }
                
                // Routing timeline
                item {
                    RoutingTimeline(attempts = payment.attempts)
                }
                
                // Actions
                item {
                    PaymentActions(
                        payment = payment,
                        onCapture = { viewModel.capture(it) },
                        onVoid = { viewModel.void() },
                        onRefund = { viewModel.refund(it) }
                    )
                }
            }
        }
    }
}
```

### Gateway Profile Section

```kotlin
@Composable
fun GatewayProfileSection(payment: PaymentIntent) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(
                text = stringResource(R.string.payment_gateway),
                style = MaterialTheme.typography.titleMedium
            )
            
            Spacer(modifier = Modifier.height(12.dp))
            
            // Gateway link
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable { navigateToConnectorDetail(payment.gatewayProfileId) },
                verticalAlignment = Alignment.CenterVertically
            ) {
                Icon(
                    imageVector = Icons.Default.AccountBalance,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary
                )
                Spacer(modifier = Modifier.width(8.dp))
                Text(
                    text = payment.connectorName,
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.primary
                )
                Icon(
                    imageVector = Icons.Default.ChevronRight,
                    contentDescription = null,
                    modifier = Modifier.size(20.dp)
                )
            }
            
            Spacer(modifier = Modifier.height(8.dp))
            
            // Rotation info
            Row {
                Text(
                    text = stringResource(R.string.rotation_strategy),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                Spacer(modifier = Modifier.weight(1f))
                Text(
                    text = payment.gatewayRotationStrategy,
                    style = MaterialTheme.typography.bodySmall
                )
            }
            
            // Fee breakdown
            if (payment.fees != null) {
                Spacer(modifier = Modifier.height(12.dp))
                FeeBreakdown(fees = payment.fees, amount = payment.amount)
            }
        }
    }
}

@Composable
fun FeeBreakdown(fees: FeeBreakdown, amount: Money) {
    Column {
        FeeRow(label = stringResource(R.string.base_amount), amount = amount)
        FeeRow(label = stringResource(R.string.fixed_fee), amount = fees.fixedFee)
        FeeRow(label = stringResource(R.string.percentage_fee), amount = fees.percentageFee)
        if (fees.crossBorderFee.amountMinorUnits > 0) {
            FeeRow(label = stringResource(R.string.cross_border_fee), amount = fees.crossBorderFee)
        }
        HorizontalDivider(modifier = Modifier.padding(vertical = 4.dp))
        FeeRow(
            label = stringResource(R.string.total_fee),
            amount = fees.totalFee,
            fontWeight = FontWeight.Bold
        )
        FeeRow(
            label = stringResource(R.string.net_amount),
            amount = Money(amount.amountMinorUnits - fees.totalFee.amountMinorUnits, amount.currency),
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.primary
        )
    }
}

@Composable
fun FeeRow(label: String, amount: Money, fontWeight: FontWeight = FontWeight.Normal, color: Color = Color.Unspecified) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween
    ) {
        Text(text = label, style = MaterialTheme.typography.bodyMedium, color = color)
        MoneyDisplay(amount = amount, fontWeight = fontWeight, color = color)
    }
}
```

---

## 3. Routing Timeline

```kotlin
@Composable
fun RoutingTimeline(attempts: List<RoutingAttempt>) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(
                text = stringResource(R.string.routing_timeline),
                style = MaterialTheme.typography.titleMedium
            )
            
            Spacer(modifier = Modifier.height(12.dp))
            
            attempts.forEachIndexed { index, attempt ->
                RoutingAttemptRow(
                    attempt = attempt,
                    isLast = index == attempts.lastIndex
                )
            }
        }
    }
}

@Composable
fun RoutingAttemptRow(attempt: RoutingAttempt, isLast: Boolean) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.Top
    ) {
        // Timeline indicator
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            modifier = Modifier.width(32.dp)
        ) {
            Box(
                modifier = Modifier
                    .size(12.dp)
                    .background(
                        if (attempt.approved) Color(0xFF22C55E) else Color(0xFFEF4444),
                        CircleShape
                    )
            )
            if (!isLast) {
                Box(
                    modifier = Modifier
                        .width(2.dp)
                        .height(40.dp)
                        .background(MaterialTheme.colorScheme.outline)
                )
            }
        }
        
        Spacer(modifier = Modifier.width(8.dp))
        
        // Attempt details
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = attempt.connectorName,
                style = MaterialTheme.typography.bodyLarge,
                fontWeight = FontWeight.Medium
            )
            
            if (!attempt.approved) {
                Text(
                    text = attempt.declineReason ?: "Unknown error",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error
                )
            }
            
            Row {
                Text(
                    text = "${attempt.latencyMs}ms",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                if (attempt.fee != null) {
                    Spacer(modifier = Modifier.width(8.dp))
                    MoneyDisplay(amount = attempt.fee, style = MaterialTheme.typography.bodySmall)
                }
            }
        }
        
        // Status badge
        StatusBadge(
            status = if (attempt.approved) "Approved" else "Declined",
            color = if (attempt.approved) "green" else "red"
        )
    }
}
```

---

## 4. Create Payment Screen

```kotlin
@Composable
fun CreatePaymentScreen(viewModel: CreatePaymentViewModel = hiltViewModel()) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.create_payment)) },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.Default.ArrowBack, contentDescription = "Back")
                    }
                }
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .padding(padding)
                .padding(16.dp)
                .verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(16.dp)
        ) {
            // Amount input
            AmountInput(
                amount = uiState.amount,
                currency = uiState.currency,
                onAmountChange = viewModel::updateAmount,
                onCurrencyChange = viewModel::updateCurrency
            )
            
            // Gateway selector (optional)
            GatewaySelector(
                gateways = uiState.availableGateways,
                selected = uiState.selectedGateway,
                onSelect = viewModel::selectGateway,
                showFees = true,
                showLimits = true
            )
            
            // Purpose
            SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
                SegmentedButton(
                    selected = uiState.purpose == PaymentPurpose.Payment,
                    onClick = { viewModel.updatePurpose(PaymentPurpose.Payment) },
                    shape = SegmentedButtonDefaults.itemShape(index = 0, count = 2)
                ) {
                    Text(stringResource(R.string.payment))
                }
                SegmentedButton(
                    selected = uiState.purpose == PaymentPurpose.CardVerification,
                    onClick = { viewModel.updatePurpose(PaymentPurpose.CardVerification) },
                    shape = SegmentedButtonDefaults.itemShape(index = 1, count = 2)
                ) {
                    Text(stringResource(R.string.card_verification))
                }
            }
            
            // Create button
            Button(
                onClick = { viewModel.createPayment() },
                modifier = Modifier.fillMaxWidth(),
                enabled = uiState.isValid && !uiState.isLoading
            ) {
                if (uiState.isLoading) {
                    CircularProgressIndicator(modifier = Modifier.size(20.dp))
                } else {
                    Text(stringResource(R.string.create_payment))
                }
            }
        }
    }
}
```

### Amount Input

```kotlin
@Composable
fun AmountInput(
    amount: String,
    currency: String,
    onAmountChange: (String) -> Unit,
    onCurrencyChange: (String) -> Unit
) {
    OutlinedTextField(
        value = amount,
        onValueChange = onAmountChange,
        label = { Text(stringResource(R.string.amount)) },
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
        modifier = Modifier.fillMaxWidth(),
        prefix = {
            Text(text = currency)
        }
    )
}
```
