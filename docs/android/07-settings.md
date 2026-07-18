# 07 — Settings

## 1. Settings Screen

```kotlin
@Composable
fun SettingsScreen(viewModel: SettingsViewModel = hiltViewModel()) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()

    LazyColumn(
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        // API Keys
        item {
            SettingsItem(
                icon = Icons.Default.Key,
                title = stringResource(R.string.api_keys),
                subtitle = stringResource(R.string.manage_api_keys),
                onClick = { navigateToApiKeys() }
            )
        }
        
        // Users
        item {
            SettingsItem(
                icon = Icons.Default.People,
                title = stringResource(R.string.users),
                subtitle = stringResource(R.string.manage_users),
                onClick = { navigateToUsers() }
            )
        }
        
        // Routing
        item {
            SettingsItem(
                icon = Icons.Default.Route,
                title = stringResource(R.string.routing_policy),
                subtitle = stringResource(R.string.configure_routing),
                onClick = { navigateToRouting() }
            )
        }
        
        // Compliance
        item {
            SettingsItem(
                icon = Icons.Default.Security,
                title = stringResource(R.string.compliance),
                subtitle = stringResource(R.string.kyb_aml_audit),
                onClick = { navigateToCompliance() }
            )
        }
        
        // Notifications
        item {
            SettingsItem(
                icon = Icons.Default.Notifications,
                title = stringResource(R.string.notifications),
                subtitle = stringResource(R.string.notification_preferences),
                onClick = { navigateToNotifications() }
            )
        }
        
        // Language
        item {
            SettingsItem(
                icon = Icons.Default.Language,
                title = stringResource(R.string.language),
                subtitle = uiState.currentLanguage,
                onClick = { viewModel.showLanguageDialog() }
            )
        }
        
        // Logout
        item {
            SettingsItem(
                icon = Icons.Default.Logout,
                title = stringResource(R.string.logout),
                onClick = { viewModel.logout() },
                color = MaterialTheme.colorScheme.error
            )
        }
    }
}

@Composable
fun SettingsItem(
    icon: ImageVector,
    title: String,
    subtitle: String? = null,
    onClick: () -> Unit,
    color: Color = MaterialTheme.colorScheme.onSurface
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
    ) {
        Row(
            modifier = Modifier.padding(16.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = icon,
                contentDescription = null,
                tint = color
            )
            Spacer(modifier = Modifier.width(16.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = title,
                    style = MaterialTheme.typography.bodyLarge,
                    color = color
                )
                subtitle?.let {
                    Text(
                        text = it,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }
            }
            Icon(
                imageVector = Icons.Default.ChevronRight,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant
            )
        }
    }
}
```

---

## 2. API Keys

```kotlin
@Composable
fun ApiKeysScreen(viewModel: ApiKeysViewModel = hiltViewModel()) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.api_keys)) },
                actions = {
                    IconButton(onClick = { viewModel.showCreateDialog() }) {
                        Icon(Icons.Default.Add, contentDescription = "Create Key")
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
            items(uiState.apiKeys) { key ->
                ApiKeyCard(
                    apiKey = key,
                    onRotate = { viewModel.rotateKey(key.id) },
                    onRevoke = { viewModel.revokeKey(key.id) }
                )
            }
        }
    }
}

@Composable
fun ApiKeyCard(
    apiKey: ApiKey,
    onRotate: () -> Unit,
    onRevoke: () -> Unit
) {
    Card(modifier = Modifier.fillMaxWidth()) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Text(
                    text = apiKey.name,
                    style = MaterialTheme.typography.titleSmall
                )
                StatusBadge(status = apiKey.status)
            }
            
            Spacer(modifier = Modifier.height(8.dp))
            
            // Key prefix (masked)
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Icon(
                    imageVector = Icons.Default.Key,
                    contentDescription = null,
                    modifier = Modifier.size(16.dp)
                )
                Spacer(modifier = Modifier.width(8.dp))
                Text(
                    text = apiKey.keyPrefix,
                    fontFamily = FontFamily.Monospace,
                    style = MaterialTheme.typography.bodyMedium
                )
                IconButton(onClick = { copyToClipboard(apiKey.keyPrefix) }) {
                    Icon(Icons.Default.ContentCopy, contentDescription = "Copy")
                }
            }
            
            // Scopes
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(4.dp)
            ) {
                apiKey.scopes.forEach { scope ->
                    Surface(
                        shape = RoundedCornerShape(4.dp),
                        color = MaterialTheme.colorScheme.secondaryContainer
                    ) {
                        Text(
                            text = scope,
                            modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp),
                            style = MaterialTheme.typography.labelSmall
                        )
                    }
                }
            }
            
            // Expiry
            Text(
                text = stringResource(R.string.expires, apiKey.expiresAt.formatRelative()),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )
            
            Spacer(modifier = Modifier.height(8.dp))
            
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                OutlinedButton(onClick = onRotate, modifier = Modifier.weight(1f)) {
                    Text(stringResource(R.string.rotate))
                }
                OutlinedButton(
                    onClick = onRevoke,
                    modifier = Modifier.weight(1f),
                    colors = ButtonDefaults.outlinedButtonColors(
                        contentColor = MaterialTheme.colorScheme.error
                    )
                ) {
                    Text(stringResource(R.string.revoke))
                }
            }
        }
    }
}
```
