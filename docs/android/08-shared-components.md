# 08 — Shared Components

## 1. Status Badge

```kotlin
@Composable
fun StatusBadge(
    status: String,
    color: String? = null,
    pulse: Boolean = false
) {
    val badgeColor = when (color) {
        "green" -> Color(0xFF22C55E)
        "red" -> Color(0xFFEF4444)
        "yellow" -> Color(0xFFEAB308)
        "blue" -> Color(0xFF3B82F6)
        "orange" -> Color(0xFFF97316)
        "gray" -> Color(0xFF6B7280)
        else -> MaterialTheme.colorScheme.surfaceVariant
    }

    Surface(
        shape = RoundedCornerShape(12.dp),
        color = badgeColor.copy(alpha = 0.1f)
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            if (pulse) {
                Box(
                    modifier = Modifier
                        .size(6.dp)
                        .background(badgeColor, CircleShape)
                        .animateContentSize()
                )
                Spacer(modifier = Modifier.width(4.dp))
            }
            Text(
                text = status,
                color = badgeColor,
                style = MaterialTheme.typography.labelSmall
            )
        }
    }
}
```

---

## 2. Money Display

```kotlin
@Composable
fun MoneyDisplay(
    amount: Money,
    fontWeight: FontWeight = FontWeight.Normal,
    color: Color = Color.Unspecified,
    style: TextStyle = MaterialTheme.typography.bodyMedium
) {
    val formatted = remember(amount) {
        NumberFormat.getCurrencyInstance(Locale("ar", "AE")).apply {
            currency = Currency.getInstance(amount.currency)
            minimumFractionDigits = getCurrencyPrecision(amount.currency)
            maximumFractionDigits = getCurrencyPrecision(amount.currency)
        }.format(amount.amountMinorUnits.toDouble() / Math.pow(10.0, getCurrencyPrecision(amount.currency).toDouble()))
    }

    Text(
        text = formatted,
        style = style,
        fontWeight = fontWeight,
        color = color,
        fontFamily = FontFamily.Monospace
    )
}

fun getCurrencyPrecision(currency: String): Int {
    return when (currency) {
        "BHD", "KWD" -> 3
        "JPY" -> 0
        else -> 2
    }
}
```

---

## 3. Loading State

```kotlin
@Composable
fun LoadingState(
    isLoading: Boolean,
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit
) {
    Box(modifier = modifier) {
        content()
        if (isLoading) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .background(MaterialTheme.colorScheme.surface.copy(alpha = 0.8f)),
                contentAlignment = Alignment.Center
            ) {
                CircularProgressIndicator()
            }
        }
    }
}

@Composable
fun TableSkeleton(rows: Int = 5, columns: Int = 5) {
    Column(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        repeat(rows) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                repeat(columns) {
                    Skeleton(
                        modifier = Modifier
                            .weight(1f)
                            .height(16.dp)
                            .clip(RoundedCornerShape(4.dp))
                    )
                }
            }
        }
    }
}
```

---

## 4. Empty State

```kotlin
@Composable
fun EmptyState(
    icon: ImageVector,
    title: String,
    description: String,
    actionText: String? = null,
    onAction: (() -> Unit)? = null
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(32.dp),
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            modifier = Modifier.size(64.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f)
        )
        
        Spacer(modifier = Modifier.height(16.dp))
        
        Text(
            text = title,
            style = MaterialTheme.typography.titleMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
        
        Spacer(modifier = Modifier.height(8.dp))
        
        Text(
            text = description,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.7f),
            textAlign = TextAlign.Center
        )
        
        if (actionText != null && onAction != null) {
            Spacer(modifier = Modifier.height(16.dp))
            Button(onClick = onAction) {
                Text(actionText)
            }
        }
    }
}
```

---

## 5. Pull-to-Refresh

```kotlin
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PullToRefreshBox(
    isRefreshing: Boolean,
    onRefresh: () -> Unit,
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit
) {
    val pullRefreshState = rememberPullToRefreshState()

    Box(modifier = modifier) {
        content()
        
        PullToRefreshContainer(
            state = pullRefreshState,
            modifier = Modifier.align(Alignment.TopCenter)
        )
    }

    LaunchedEffect(isRefreshing) {
        if (isRefreshing) {
            pullRefreshState.animateToThreshold()
        } else {
            pullRefreshState.animateToHidden()
        }
    }

    LaunchedEffect(pullRefreshState) {
        if (pullRefreshState.state == PullToRefreshState.Threshold) {
            onRefresh()
        }
    }
}
```

---

## 6. Date Formatting

```kotlin
fun DateTime.formatRelative(): String {
    val now = Clock.System.now()
    val diff = now - this

    return when {
        diff < 1.minutes -> stringResource(R.string.just_now)
        diff < 1.hours -> stringResource(R.string.minutes_ago, diff.inWholeMinutes)
        diff < 1.days -> stringResource(R.string.hours_ago, diff.inWholeHours)
        diff < 7.days -> stringResource(R.string.days_ago, diff.inWholeDays)
        else -> this.formatShort()
    }
}

fun DateTime.formatShort(): String {
    return this.toLocalDateTime(TimeZone.currentSystemDefault()).let { dt ->
        "${dt.monthNumber}/${dt.dayOfMonth}/${dt.year}"
    }
}
```
