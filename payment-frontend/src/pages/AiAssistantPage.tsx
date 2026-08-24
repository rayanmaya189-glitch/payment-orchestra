import { useState, useRef, useEffect } from 'react';
import { 
  Send, 
  Bot, 
  User, 
  Sparkles, 
  RefreshCw, 
  Copy, 
  Check,
} from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { clsx } from 'clsx';

interface Message {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
  sources?: string[];
}

const sampleQuestions = [
  "What's my current success rate?",
  "Show me failed transactions from yesterday",
  "Which gateway has the highest success rate?",
  "How much volume did we process today?",
  "What are the top decline reasons?",
  "Compare performance of Stripe vs Checkout.com",
];

const sampleResponses: Record<string, string> = {
  "What's my current success rate?": "Based on your recent transaction data:\n\n• **Overall Success Rate:** 97.8%\n• **Last 24 hours:** 98.2%\n• **Last 7 days:** 97.5%\n\nYour success rate has improved by 0.3% compared to last week. The main factors contributing to this improvement are:\n\n1. **Reduced timeouts** - Gateway timeouts decreased by 15%\n2. **Better routing** - Smart routing is now handling 40% more transactions\n3. **Lower decline rates** - Insufficient funds declines dropped by 8%",
  
  "Show me failed transactions from yesterday": "Here's a summary of failed transactions from yesterday:\n\n• **Total Failed:** 142 transactions\n• **Total Amount:** $12,450.00\n• **Top Decline Reasons:**\n  1. Insufficient Funds (45%)\n  2. Do Not Honor (28%)\n  3. Expired Card (15%)\n  4. Fraud Suspected (8%)\n  5. Other (4%)\n\n**Recommendation:** Consider enabling automatic failover for 'Insufficient Funds' declines to route to alternative gateways.",
  
  "Which gateway has the highest success rate?": "Gateway Performance Summary:\n\n1. **Stripe** - 98.5% success rate (12,450 transactions)\n2. **Checkout.com** - 97.8% success rate (8,320 transactions)\n3. **Adyen** - 96.2% success rate (5,670 transactions)\n4. **Razorpay** - 95.8% success rate (3,210 transactions)\n\n**Top Performer:** Stripe with 98.5% success rate\n**Most Improved:** Adyen (+2.1% from last month)\n\nI recommend prioritizing Stripe for high-value transactions and using Adyen as a fallback for Indian payment methods.",
};

export function AiAssistantPage() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [inputValue, setInputValue] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [copiedMessageId, setCopiedMessageId] = useState<string | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  const handleSendMessage = async (content: string) => {
    if (!content.trim()) return;

    // Add user message
    const userMessage: Message = {
      id: Date.now().toString(),
      role: 'user',
      content: content.trim(),
      timestamp: new Date(),
    };
    setMessages((prev) => [...prev, userMessage]);
    setInputValue('');
    setIsLoading(true);

    // Simulate AI response
    setTimeout(() => {
      const response = sampleResponses[content] || 
        `I received your question: "${content}"\n\nI'm analyzing your payment data to provide insights. In a production environment, this would be powered by the AI assistant service using RAG (Retrieval-Augmented Generation) over your transaction data.\n\nKey metrics I can help you with:\n• Transaction success rates\n• Gateway performance comparison\n• Decline reason analysis\n• Volume and revenue trends\n• Routing optimization suggestions`;
      
      const assistantMessage: Message = {
        id: (Date.now() + 1).toString(),
        role: 'assistant',
        content: response,
        timestamp: new Date(),
        sources: ['payment_intents', 'gateway_profiles', 'routing_policies'],
      };
      setMessages((prev) => [...prev, assistantMessage]);
      setIsLoading(false);
    }, 1500);
  };

  const handleCopyMessage = async (content: string, messageId: string) => {
    await navigator.clipboard.writeText(content);
    setCopiedMessageId(messageId);
    setTimeout(() => setCopiedMessageId(null), 2000);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage(inputValue);
    }
  };

  return (
    <div className="flex flex-col h-[calc(100vh-8rem)]">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-3">
          <div className="p-2 bg-primary-100 rounded-lg">
            <Sparkles className="w-5 h-5 text-primary-600" />
          </div>
          <div>
            <h1 className="text-2xl font-bold text-gray-900">AI Assistant</h1>
            <p className="text-sm text-gray-500">Ask questions about your payment data</p>
          </div>
        </div>
      </div>

      {/* Chat Container */}
      <Card className="flex-1 flex flex-col overflow-hidden">
        {/* Messages */}
        <div className="flex-1 overflow-y-auto p-4 space-y-4">
          {messages.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-full">
              <div className="p-4 bg-primary-100 rounded-full mb-4">
                <Bot className="w-12 h-12 text-primary-600" />
              </div>
              <h3 className="text-lg font-medium text-gray-900 mb-2">How can I help you?</h3>
              <p className="text-sm text-gray-500 mb-6 text-center max-w-md">
                I can answer questions about your transactions, analyze performance, 
                and provide insights to help optimize your payment operations.
              </p>
              
              {/* Sample Questions */}
              <div className="grid grid-cols-1 md:grid-cols-2 gap-2 max-w-2xl">
                {sampleQuestions.map((question, index) => (
                  <button
                    key={index}
                    onClick={() => handleSendMessage(question)}
                    className="p-3 text-left text-sm bg-gray-50 hover:bg-gray-100 rounded-lg transition-colors"
                  >
                    {question}
                  </button>
                ))}
              </div>
            </div>
          ) : (
            <>
              {messages.map((message) => (
                <div
                  key={message.id}
                  className={clsx(
                    'flex gap-3',
                    message.role === 'user' ? 'justify-end' : 'justify-start'
                  )}
                >
                  {message.role === 'assistant' && (
                    <div className="flex-shrink-0 w-8 h-8 bg-primary-100 rounded-full flex items-center justify-center">
                      <Bot className="w-4 h-4 text-primary-600" />
                    </div>
                  )}
                  
                  <div
                    className={clsx(
                      'max-w-[80%] rounded-lg p-4',
                      message.role === 'user'
                        ? 'bg-primary-600 text-white'
                        : 'bg-gray-100 text-gray-900'
                    )}
                  >
                    <div className="prose prose-sm max-w-none">
                      {message.content.split('\n').map((line, i) => (
                        <p key={i} className="mb-2 last:mb-0">
                          {line.split('**').map((part, j) =>
                            j % 2 === 1 ? <strong key={j}>{part}</strong> : part
                          )}
                        </p>
                      ))}
                    </div>
                    
                    {message.sources && (
                      <div className="mt-3 pt-3 border-t border-gray-200">
                        <p className="text-xs text-gray-500 mb-1">Sources:</p>
                        <div className="flex flex-wrap gap-1">
                          {message.sources.map((source) => (
                            <span
                              key={source}
                              className="px-2 py-0.5 text-xs bg-gray-200 text-gray-600 rounded"
                            >
                              {source}
                            </span>
                          ))}
                        </div>
                      </div>
                    )}
                    
                    <div className="flex items-center justify-between mt-3">
                      <span className="text-xs text-gray-400">
                        {message.timestamp.toLocaleTimeString()}
                      </span>
                      <button
                        onClick={() => handleCopyMessage(message.content, message.id)}
                        className="p-1 text-gray-400 hover:text-gray-600"
                      >
                        {copiedMessageId === message.id ? (
                          <Check className="w-4 h-4 text-success-500" />
                        ) : (
                          <Copy className="w-4 h-4" />
                        )}
                      </button>
                    </div>
                  </div>
                  
                  {message.role === 'user' && (
                    <div className="flex-shrink-0 w-8 h-8 bg-gray-200 rounded-full flex items-center justify-center">
                      <User className="w-4 h-4 text-gray-600" />
                    </div>
                  )}
                </div>
              ))}
              
              {isLoading && (
                <div className="flex gap-3">
                  <div className="flex-shrink-0 w-8 h-8 bg-primary-100 rounded-full flex items-center justify-center">
                    <Bot className="w-4 h-4 text-primary-600" />
                  </div>
                  <div className="bg-gray-100 rounded-lg p-4">
                    <div className="flex items-center gap-2">
                      <RefreshCw className="w-4 h-4 text-gray-400 animate-spin" />
                      <span className="text-sm text-gray-500">Analyzing your data...</span>
                    </div>
                  </div>
                </div>
              )}
            </>
          )}
          <div ref={messagesEndRef} />
        </div>

        {/* Input */}
        <div className="p-4 border-t border-gray-200">
          <div className="flex gap-2">
            <input
              type="text"
              value={inputValue}
              onChange={(e) => setInputValue(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Ask about your payments, transactions, or performance..."
              className="flex-1 px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-primary-500"
              disabled={isLoading}
            />
            <button
              onClick={() => handleSendMessage(inputValue)}
              disabled={!inputValue.trim() || isLoading}
              className="btn-primary px-4"
            >
              <Send className="w-4 h-4" />
            </button>
          </div>
          <p className="text-xs text-gray-400 mt-2">
            Powered by AI with RAG over your payment data. Responses are based on your transaction history.
          </p>
        </div>
      </Card>
    </div>
  );
}
