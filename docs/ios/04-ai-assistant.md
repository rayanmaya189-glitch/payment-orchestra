# 04 — AI Assistant

## 1. Assistant Chat Interface

```swift
struct AssistantView: View {
    @StateObject var viewModel = Container.assistantVM()
    @State private var inputText = ""
    @FocusState private var isInputFocused: Bool
    
    var body: some View {
        VStack(spacing: 0) {
            // Messages
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(spacing: 12) {
                        ForEach(viewModel.messages) { message in
                            ChatBubble(message: message)
                                .id(message.id)
                        }
                        
                        if viewModel.isGenerating {
                            TypingIndicator()
                                .id("typing")
                        }
                    }
                    .padding()
                }
                .onChange(of: viewModel.messages.count) { _ in
                    withAnimation {
                        proxy.scrollTo(viewModel.messages.last?.id ?? "typing", anchor: .bottom)
                    }
                }
            }
            
            // Input
            ChatInputBar(
                text: $inputText,
                onSend: {
                    viewModel.sendMessage(inputText)
                    inputText = ""
                },
                onVoice: { viewModel.startVoiceInput() },
                isLoading: viewModel.isGenerating
            )
        }
        .navigationTitle("AI Assistant")
    }
}

struct ChatBubble: View {
    let message: ChatMessage
    
    var body: some View {
        HStack(alignment: .top, spacing: 8) {
            if message.role == .assistant {
                Image(systemName: "sparkles")
                    .foregroundStyle(.blue)
                    .frame(width: 24, height: 24)
            }
            
            VStack(alignment: message.role == .user ? .trailing : .leading, spacing: 4) {
                Text(message.content)
                    .padding(12)
                    .background(message.role == .user ? Color.blue : Color(.systemGray6))
                    .foregroundStyle(message.role == .user ? .white : .primary)
                    .clipShape(RoundedRectangle(cornerRadius: 16))
                
                // Citations
                if !message.citations.isEmpty {
                    CitationsView(citations: message.citations)
                }
                
                // Feedback
                if message.role == .assistant {
                    FeedbackButtons(messageId: message.id, feedback: message.feedback) { feedback in
                        viewModel.submitFeedback(messageId: message.id, feedback: feedback)
                    }
                }
            }
            
            if message.role == .user {
                Spacer()
            }
        }
    }
}
```

### Citations

```swift
struct CitationsView: View {
    let citations: [Citation]
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Sources")
                .font(.caption2)
                .foregroundStyle(.secondary)
            
            ForEach(citations) { citation in
                Button(action: { navigateToSource(citation) }) {
                    HStack(spacing: 4) {
                        Image(systemName: iconForSource(citation.sourceType))
                            .font(.caption2)
                        Text(citation.excerpt.prefix(50).appending("..."))
                            .font(.caption2)
                            .lineLimit(1)
                    }
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(Color.blue.opacity(0.1))
                    .clipShape(Capsule())
                }
            }
        }
    }
    
    private func iconForSource(_ type: String) -> String {
        switch type {
        case "transaction": return "doc.text"
        case "document": return "doc"
        case "summary": return "list.bullet"
        default: return "questionmark"
        }
    }
}
```

---

## 2. Voice Recognition

```swift
import Speech

class VoiceRecognizer: ObservableObject {
    private let speechRecognizer = SFSpeechRecognizer(locale: Locale(identifier: "ar-AE"))
    private var recognitionRequest: SFSpeechAudioBufferRecognitionRequest?
    private var recognitionTask: SFSpeechRecognitionTask?
    private let audioEngine = AVAudioEngine()
    
    func startListening(completion: @escaping (String) -> Void) {
        recognitionRequest = SFSpeechAudioBufferRecognitionRequest()
        guard let request = recognitionRequest else { return }
        
        request.shouldReportPartialResults = true
        
        let inputNode = audioEngine.inputNode
        let recordingFormat = inputNode.outputFormat(forBus: 0)
        inputNode.installTap(onBus: 0, bufferSize: 1024, format: recordingFormat) { buffer, _ in
            request.append(buffer)
        }
        
        audioEngine.prepare()
        try? audioEngine.start()
        
        recognitionTask = speechRecognizer?.recognitionTask(with: request) { result, _ in
            if let result = result {
                DispatchQueue.main.async {
                    completion(result.bestTranscription.formattedString)
                }
            }
        }
    }
    
    func stopListening() {
        audioEngine.stop()
        audioEngine.inputNode.removeTap(onBus: 0)
        recognitionRequest = nil
        recognitionTask = nil
    }
}
```

---

## 3. Chat Input Bar

```swift
struct ChatInputBar: View {
    @Binding var text: String
    let onSend: () -> Void
    let onVoice: () -> Void
    let isLoading: Bool
    
    var body: some View {
        HStack(spacing: 8) {
            Button(action: onVoice) {
                Image(systemName: "mic.fill")
                    .foregroundStyle(.blue)
            }
            
            TextField("Ask about your payments...", text: $text)
                .textFieldStyle(.plain)
                .lineLimit(3)
            
            Button(action: onSend) {
                if isLoading {
                    ProgressView()
                        .scaleEffect(0.8)
                } else {
                    Image(systemName: "arrow.up.circle.fill")
                        .foregroundStyle(text.isEmpty ? .gray : .blue)
                }
            }
            .disabled(text.isEmpty || isLoading)
        }
        .padding()
        .background(.bar)
    }
}
```
