# 01 — Dashboard

## 1. Main Dashboard (`DashboardScreen`)

### Compose Layout

```kotlin
@Composable
fun DashboardScreen(viewModel: DashboardViewModel = hiltViewModel()) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()
    val pullRefreshState = rememberPullToRefreshState()

    Scaffold(
        topBar = { DashboardTopBar() },
        bottomBar = { BottomNavBar() }
    ) { padding ->
        PullToRefreshBox(
            state = pullRefreshState,
            onRefresh = { viewModel.refresh() }
        ) {
            LazyColumn(
                modifier = Modifier.padding(padding),
                contentPadding = PaddingValues(16.dp)
            ) {
                // Stats cards
                item {
                    StatsGrid(
                        stats = uiState.stats,
                        onCardClick = { stat -> navigateToDetail(stat) }
                    )
                }
                
                // Charts
                item {
                    ChartsSection(
                        hourlyRates = uiState.hourlyRates,
                        declineBreakdown = uiState.declineBreakdown
                    )
                }
                
                // Recent activity
                item {
                    RecentActivityTable(
                        transactions = uiState.recentTransactions,
                        onTransactionClick = { navigateToPaymentDetail(it) }
                    )
                }
            }
        }
    }
}
```

### Stats Cards

```kotlin
@Composable
fun StatsGrid(stats: DashboardStats) {
    LazyVerticalGrid(
        columns = GridCells.Fixed(2),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        item {
            StatCard(
                title = stringResource(R.string.transactions_today),
                value = stats.totalTransactions.format(),
                trend = stats.txTrend,
                icon = Icons.Default.Receipt
            )
        }
        item {
            StatCard(
                title = stringResource(R.string.auth_rate),
                value = "${stats.authRate}%",
                trend = stats.authTrend,
                icon = Icons.Default.CheckCircle
            )
        }
        item {
            StatCard(
                title = stringResource(R.string.revenue_recovered),
                value = stats.recoveredAmount.formatMoney("AED"),
                trend = stats.recoveredTrend,
                icon = Icons.Default.TrendingUp
            )
        }
        item {
            StatCard(
                title = stringResource(R.string.pending_settlements),
                value = stats.pendingSettlements.toString(),
                icon = Icons.Default.Schedule
            )
        }
    }
}
```

### Real-time Updates

```kotlin
@HiltViewModel
class DashboardViewModel @Inject constructor(
    private val paymentRepository: PaymentRepository,
    private val webSocketManager: PaymentWebSocket
) : ViewModel() {

    private val _uiState = MutableStateFlow(DashboardUiState())
    val uiState: StateFlow<DashboardUiState> = _uiState.asStateFlow()

    init {
        // Connect to WebSocket for real-time updates
        viewModelScope.launch {
            webSocketManager.messages.collect { message ->
                when (message.type) {
                    "payment_status_changed" -> refreshStats()
                    "settlement_matched" -> refreshStats()
                    "aml_alert" -> showAlert(message)
                }
            }
        }
    }

    fun refresh() {
        viewModelScope.launch {
            _uiState.update { it.copy(isRefreshing = true) }
            val stats = paymentRepository.getDashboardStats()
            _uiState.update { it.copy(stats = stats, isRefreshing = false) }
        }
    }
}
```

---

## 2. Stat Card Component

```kotlin
@Composable
fun StatCard(
    title: String,
    value: String,
    trend: Float? = null,
    icon: ImageVector,
    onClick: () -> Unit = {}
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surface
        )
    ) {
        Column(
            modifier = Modifier.padding(16.dp)
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Icon(
                    imageVector = icon,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary
                )
                trend?.let { TrendBadge(trend = it) }
            }
            
            Spacer(modifier = Modifier.height(12.dp))
            
            Text(
                text = value,
                style = MaterialTheme.typography.headlineMedium,
                fontWeight = FontWeight.Bold
            )
            
            Text(
                text = title,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}

@Composable
fun TrendBadge(trend: Float) {
    val color = when {
        trend > 0 -> Color(0xFF22C55E)  // green
        trend < 0 -> Color(0xFFEF4444)  // red
        else -> Color(0xFF6B7280)       // gray
    }
    
    Row(verticalAlignment = Alignment.CenterVertically) {
        Icon(
            imageVector = if (trend > 0) Icons.Default.TrendingUp else Icons.Default.TrendingDown,
            contentDescription = null,
            tint = color,
            modifier = Modifier.size(16.dp)
        )
        Text(
            text = "${abs(trend)}%",
            color = color,
            style = MaterialTheme.typography.labelSmall
        )
    }
}
```

---

## 3. Charts

```kotlin
@Composable
fun AuthRateChart(data: List<HourlyRate>) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(
                text = stringResource(R.string.authorization_rate),
                style = MaterialTheme.typography.titleMedium
            )
            
            Spacer(modifier = Modifier.height(16.dp))
            
            // Using a chart library like Vico or YCharts
            LineChart(
                data = data.map { Point(it.hour.toFloat(), it.rate) },
                lineColor = MaterialTheme.colorScheme.primary,
                modifier = Modifier
                    .fillMaxWidth()
                    .height(200.dp)
            )
        }
    }
}

@Composable
fun DeclineReasonPieChart(data: List<DeclineReasonCount>) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(
                text = stringResource(R.string.decline_reasons),
                style = MaterialTheme.typography.titleMedium
            )
            
            Spacer(modifier = Modifier.height(16.dp))
            
            PieChart(
                data = data.map { PieSlice(it.reason, it.count, it.color) },
                modifier = Modifier.size(200.dp)
            )
            
            // Legend
            data.forEach { item ->
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Box(
                        modifier = Modifier
                            .size(12.dp)
                            .background(item.color, CircleShape)
                    )
                    Spacer(modifier = Modifier.width(8.dp))
                    Text(text = item.reason, style = MaterialTheme.typography.bodySmall)
                    Spacer(modifier = Modifier.weight(1f))
                    Text(text = item.count.toString(), style = MaterialTheme.typography.bodySmall)
                }
            }
        }
    }
}
```

---

## 4. Recent Activity Table

```kotlin
@Composable
fun RecentActivityTable(
    transactions: List<PaymentIntent>,
    onTransactionClick: (PaymentIntent) -> Unit
) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp)) {
            Text(
                text = stringResource(R.string.recent_activity),
                style = MaterialTheme.typography.titleMedium
            )
            
            Spacer(modifier = Modifier.height(12.dp))
            
            transactions.take(10).forEach { transaction ->
                PaymentRow(
                    transaction = transaction,
                    onClick = { onTransactionClick(transaction) }
                )
                if (transaction != transactions.last()) {
                    HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))
                }
            }
        }
    }
}

@Composable
fun PaymentRow(
    transaction: PaymentIntent,
    onClick: () -> Unit
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
            .padding(vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = transaction.id.take(12) + "...",
                style = MaterialTheme.typography.bodyMedium,
                fontFamily = FontFamily.Monospace
            )
            Text(
                text = transaction.createdAt.formatRelative(),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
        
        Column(horizontalAlignment = Alignment.End) {
            MoneyDisplay(
                amount = transaction.amount,
                currency = transaction.currency
            )
            StatusBadge(status = transaction.status)
        }
    }
}
```
