# 04 — AI Assistant

## 1. Assistant Interface (`/assistant`)

### Layout

```
┌─────────────────────────────────────────────────┐
│ AI Payment Assistant                            │
├─────────────────────────────────────────────────┤
│ Session: [dropdown]                             │
├─────────────────────────────────────────────────┤
│                                                 │
│ 👤 User: Why did our authorization rate drop    │
│         yesterday for Mastercard?               │
│                                                 │
│ 🤖 Assistant: Your Mastercard authorization     │
│    rate dropped from 94.2% to 86.7% yesterday.  │
│    The primary cause was a 12% increase in      │
│    "Issuer Unavailable" declines from Acquirer B │
│    between 14:00-16:00 GST.                      │
│                                                 │
│    📎 Sources:                                   │
│    - PaymentAuthorizationAttempted (342 events)  │
│    - Hourly auth rate rollup (14:00-16:00)       │
│    - Acquirer B circuit breaker logs             │
│                                                 │
│ 👤 User: Was this related to the Acquirer B     │
│         maintenance window?                     │
│                                                 │
│ 🤖 Assistant: Yes. Acquirer B's status page     │
│    shows scheduled maintenance from 14:00-      │
│    15:30 GST. During this period, 89 of your    │
│    342 Mastercard transactions were routed to    │
│    Acquirer B and received "Issuer Unavailable"  │
│    declines. Your routing policy correctly       │
│    failed over to Acquirer A for subsequent      │
│    attempts.                                    │
│                                                 │
│    📎 Sources:                                   │
│    - Acquirer B maintenance log                  │
│    - PaymentAuthorizationAttempted (89 events)   │
│    - Routing policy configuration                │
│                                                 │
├─────────────────────────────────────────────────┤
│ [Type your question...]              [Send] 📎  │
└─────────────────────────────────────────────────┘
```

### Components

```tsx
<AssistantLayout>
  <SessionSelector sessions={sessions} onChange={handleSessionChange} />

  <ChatMessages>
    {messages.map(msg => (
      <ChatMessage
        key={msg.id}
        role={msg.role}
        content={msg.content}
        citations={msg.citations}
        feedback={msg.feedback}
        onFeedback={handleFeedback}
      />
    ))}
  </ChatMessages>

  <ChatInput
    onSend={handleSend}
    onUpload={handleFileUpload}
    isLoading={isGenerating}
    placeholder="Ask about your transactions, settlements, or reconciliation..."
  />
</AssistantLayout>
```

---

## 2. Citation Component

```tsx
<Citation sources={citations}>
  {citations.map(cite => (
    <CitationChip
      key={cite.id}
      type={cite.sourceType}    // 'transaction' | 'document' | 'summary'
      id={cite.sourceId}
      excerpt={cite.excerpt}
      onClick={() => navigateToSource(cite)}
    />
  ))}
</Citation>

// Navigation on click:
const navigateToSource = (cite: Citation) => {
  switch (cite.sourceType) {
    case 'transaction':
      router.push(`/payments/${cite.sourceId}`);
      break;
    case 'document':
      openDocumentViewer(cite.sourceId);
      break;
    case 'summary':
      // Show summary in modal
      break;
  }
};
```

---

## 3. File Upload

```tsx
<FileUpload
  accept=".pdf,.png,.jpg"
  maxSize={10 * 1024 * 1024} // 10MB
  onUpload={async (file) => {
    const formData = new FormData();
    formData.append('file', file);
    const response = await api.post('/v1/documents', formData, {
      headers: { 'Content-Type': 'multipart/form-data' },
    });
    return response.data;
  }}
/>
```

### Document Processing States

| State | UI |
|-------|-----|
| Uploaded | Spinner with "Processing document..." |
| OcrProcessing | Progress bar with extraction status |
| OcrCompleted | Success badge, extracted fields displayed |
| OcrFailed | Error badge, retry button |

---

## 4. Feedback Mechanism

```tsx
<FeedbackButtons
  messageId={message.id}
  currentFeedback={message.feedback}
  onFeedback={async (feedback: 'positive' | 'negative') => {
    await api.post('/v1/assistant/feedback', {
      session_id: message.sessionId,
      message_id: message.id,
      feedback,
    });
  }}
/>
```

---

## 5. Real-time Streaming

```typescript
// SSE for streaming AI responses
const streamResponse = async (question: string) => {
  const eventSource = new EventSource(
    `${AI_URL}/v1/assistant/stream?question=${encodeURIComponent(question)}`
  );

  eventSource.onmessage = (event) => {
    const chunk = JSON.parse(event.data);
    setStreamingResponse(prev => prev + chunk.text);
    if (chunk.citations) {
      setCitations(chunk.citations);
    }
  };

  eventSource.onerror = () => {
    eventSource.close();
    // Fallback to non-streaming response
  };
};
```
