# 04 — AI Assistant

## 1. Assistant Chat Screen

```kotlin
@Composable
fun AssistantScreen(viewModel: AssistantViewModel = hiltViewModel()) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()
    val listState = rememberLazyListState()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.ai_assistant)) },
                actions = {
                    IconButton(onClick = { viewModel.clearSession() }) {
                        Icon(Icons.Default.Refresh, contentDescription = "New Session")
                    }
                }
            )
        },
        bottomBar = {
            ChatInput(
                value = uiState.inputText,
                onValueChange = viewModel::updateInput,
                onSend = { viewModel.sendMessage() },
                onVoiceInput = { viewModel.startVoiceInput() },
                isLoading = uiState.isGenerating
            )
        }
    ) { padding ->
        LazyColumn(
            state = listState,
            modifier = Modifier.padding(padding),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            items(uiState.messages) { message ->
                ChatBubble(message = message)
            }
            
            if (uiState.isGenerating) {
                item {
                    TypingIndicator()
                }
            }
        }
        
        // Auto-scroll to bottom
        LaunchedEffect(uiState.messages.size) {
            listState.animateScrollToItem(uiState.messages.size - 1)
        }
    }
}
```

### Chat Bubble

```kotlin
@Composable
fun ChatBubble(message: ChatMessage) {
    val isUser = message.role == MessageRole.USER

    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = if (isUser) Arrangement.End else Arrangement.Start
    ) {
        if (!isUser) {
            Icon(
                imageVector = Icons.Default.AutoAwesome,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.primary,
                modifier = Modifier.size(32.dp)
            )
            Spacer(modifier = Modifier.width(8.dp))
        }
        
        Column(
            modifier = Modifier
                .widthIn(max = 300.dp)
                .background(
                    if (isUser) MaterialTheme.colorScheme.primary
                    else MaterialTheme.colorScheme.surfaceVariant,
                    RoundedCornerShape(16.dp)
                )
                .padding(12.dp)
        ) {
            Text(
                text = message.content,
                color = if (isUser) MaterialTheme.colorScheme.onPrimary
                        else MaterialTheme.colorScheme.onSurfaceVariant
            )
            
            // Citations
            if (message.citations.isNotEmpty()) {
                Spacer(modifier = Modifier.height(8.dp))
                CitationsList(citations = message.citations)
            }
            
            // Feedback buttons (for assistant messages)
            if (!isUser) {
                Spacer(modifier = Modifier.height(8.dp))
                FeedbackButtons(
                    messageId = message.id,
                    feedback = message.feedback,
                    onFeedback = { viewModel.submitFeedback(message.id, it) }
                )
            }
        }
    }
}
```

### Citations

```kotlin
@Composable
fun CitationsList(citations: List<Citation>) {
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(
            text = stringResource(R.string.sources),
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
        
        citations.forEach { citation ->
            CitationChip(
                citation = citation,
                onClick = { navigateToSource(citation) }
            )
        }
    }
}

@Composable
fun CitationChip(citation: Citation, onClick: () -> Unit) {
    Surface(
        onClick = onClick,
        shape = RoundedCornerShape(12.dp),
        color = MaterialTheme.colorScheme.primaryContainer
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Icon(
                imageVector = when (citation.sourceType) {
                    "transaction" -> Icons.Default.Receipt
                    "document" -> Icons.Default.Description
                    "summary" -> Icons.Default.Summarize
                },
                contentDescription = null,
                modifier = Modifier.size(14.dp),
                tint = MaterialTheme.colorScheme.primary
            )
            Spacer(modifier = Modifier.width(4.dp))
            Text(
                text = citation.excerpt.take(50),
                style = MaterialTheme.typography.labelSmall
            )
        }
    }
}
```

### Voice Input

```kotlin
@Composable
fun ChatInput(
    value: String,
    onValueChange: (String) -> Unit,
    onSend: () -> Unit,
    onVoiceInput: () -> Unit,
    isLoading: Boolean
) {
    Surface(
        modifier = Modifier.fillMaxWidth(),
        tonalElevation = 3.dp
    ) {
        Row(
            modifier = Modifier.padding(8.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            // Voice input button
            IconButton(onClick = onVoiceInput) {
                Icon(
                    imageVector = Icons.Default.Mic,
                    contentDescription = stringResource(R.string.voice_input)
                )
            }
            
            // Text input
            OutlinedTextField(
                value = value,
                onValueChange = onValueChange,
                modifier = Modifier.weight(1f),
                placeholder = { Text(stringResource(R.string.ask_about_payments)) },
                maxLines = 3
            )
            
            // Send button
            IconButton(
                onClick = onSend,
                enabled = value.isNotBlank() && !isLoading
            ) {
                if (isLoading) {
                    CircularProgressIndicator(modifier = Modifier.size(20.dp))
                } else {
                    Icon(
                        imageVector = Icons.Default.Send,
                        contentDescription = stringResource(R.string.send)
                    )
                }
            }
        }
    }
}
```

---

## 2. Voice Recognition Integration

```kotlin
class VoiceRecognizer(private val context: Context) {
    private val speechRecognizer = SpeechRecognizer.createSpeechRecognizer(context)
    
    fun startListening(
        onResult: (String) -> Unit,
        onError: (String) -> Unit
    ) {
        speechRecognizer.setRecognitionListener(object : RecognitionListener {
            override fun onResults(results: Bundle?) {
                val matches = results?.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION)
                matches?.firstOrNull()?.let { onResult(it) }
            }
            
            override fun onError(error: Int) {
                onError("Voice recognition failed: $error")
            }
            
            // ... other methods
        })
        
        val intent = Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH).apply {
            putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
            putExtra(RecognizerIntent.EXTRA_LANGUAGE, "ar-AE") // Arabic (UAE)
        }
        speechRecognizer.startListening(intent)
    }
    
    fun stopListening() {
        speechRecognizer.stopListening()
    }
}
```
