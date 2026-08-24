/**
 * Payment Orchestra iOS SDK
 *
 * Usage:
 *   let client = PaymentOrchestra(apiKey: "pk_live_...")
 *
 *   let intent = try await client.paymentIntents.create(
 *       amount: 5000,
 *       currency: "USD",
 *       purpose: .payment
 *   )
 *
 *   try await client.paymentIntents.authorize(
 *       intentId: intent.id,
 *       paymentMethodToken: "tok_visa_4242",
 *       cardScheme: "visa"
 *   )
 */

import Foundation

// MARK: - Configuration

public struct PaymentOrchestraConfig {
    public let apiKey: String
    public let environment: Environment
    public let baseUrl: URL
    public let timeout: TimeInterval

    public enum Environment {
        case production
        case sandbox
    }

    public init(
        apiKey: String,
        environment: Environment = .sandbox,
        baseUrl: URL? = nil,
        timeout: TimeInterval = 30
    ) {
        self.apiKey = apiKey
        self.environment = environment
        self.timeout = timeout

        if let baseUrl = baseUrl {
            self.baseUrl = baseUrl
        } else {
            switch environment {
            case .production:
                self.baseUrl = URL(string: "https://api.payment-orchestra.com")!
            case .sandbox:
                self.baseUrl = URL(string: "https://sandbox.payment-orchestra.com")!
            }
        }
    }
}

// MARK: - Client

public class PaymentOrchestra {
    private let config: PaymentOrchestraConfig
    private let session: URLSession

    public lazy var paymentIntents = PaymentIntentsResource(client: self)
    public lazy var gatewayProfiles = GatewayProfilesResource(client: self)
    public lazy var routingPolicies = RoutingPoliciesResource(client: self)
    public lazy var webhooks = WebhooksResource(client: self)
    public lazy var apiKeys = ApiKeysResource(client: self)

    public init(config: PaymentOrchestraConfig) {
        self.config = config

        let sessionConfig = URLSessionConfiguration.default
        sessionConfig.timeoutIntervalForRequest = config.timeout
        sessionConfig.timeoutIntervalForResource = config.timeout
        self.session = URLSession(configuration: sessionConfig)
    }

    public convenience init(apiKey: String, environment: PaymentOrchestraConfig.Environment = .sandbox) {
        self.init(config: PaymentOrchestraConfig(apiKey: apiKey, environment: environment))
    }

    // MARK: - Internal Request Methods

    func request<T: Decodable>(
        method: String,
        path: String,
        body: Encodable? = nil
    ) async throws -> T {
        let url = config.baseUrl.appendingPathComponent(path)

        var request = URLRequest(url: url)
        request.httpMethod = method
        request.setValue("Bearer \(config.apiKey)", forHTTPHeaderField: "Authorization")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("1", forHTTPHeaderField: "X-API-Version")
        request.setValue("PaymentOrchestra-SDK-iOS/0.1.0", forHTTPHeaderField: "User-Agent")

        if let body = body {
            request.httpBody = try JSONEncoder().encode(body)
        }

        let (data, response) = try await session.data(for: request)

        guard let httpResponse = response as? HTTPURLResponse else {
            throw PaymentOrchestraError.invalidResponse
        }

        guard (200...299).contains(httpResponse.statusCode) else {
            let errorResponse = try? JSONDecoder().decode(ErrorResponse.self, from: data)
            throw PaymentOrchestraError.httpError(
                status: httpResponse.statusCode,
                message: errorResponse?.detail ?? "HTTP \(httpResponse.statusCode)"
            )
        }

        return try JSONDecoder().decode(T.self, from: data)
    }

    func requestVoid(
        method: String,
        path: String,
        body: Encodable? = nil
    ) async throws {
        let url = config.baseUrl.appendingPathComponent(path)

        var request = URLRequest(url: url)
        request.httpMethod = method
        request.setValue("Bearer \(config.apiKey)", forHTTPHeaderField: "Authorization")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("1", forHTTPHeaderField: "X-API-Version")

        if let body = body {
            request.httpBody = try JSONEncoder().encode(body)
        }

        let (_, response) = try await session.data(for: request)

        guard let httpResponse = response as? HTTPURLResponse,
              (200...299).contains(httpResponse.statusCode) else {
            throw PaymentOrchestraError.invalidResponse
        }
    }
}

// MARK: - Payment Intents Resource

public class PaymentIntentsResource {
    private let client: PaymentOrchestra

    init(client: PaymentOrchestra) {
        self.client = client
    }

    public func create(
        amount: Int,
        currency: String,
        purpose: PaymentPurpose = .payment,
        idempotencyKey: String? = nil,
        metadata: [String: Any]? = nil
    ) async throws -> PaymentIntent {
        var body: [String: Any] = [
            "amount_minor_units": amount,
            "currency": currency,
            "purpose": purpose.rawValue,
        ]
        if let idempotencyKey = idempotencyKey {
            body["idempotency_key"] = idempotencyKey
        }
        if let metadata = metadata {
            body["metadata"] = metadata
        }

        let data = try JSONSerialization.data(withJSONObject: body)
        let request = try JSONDecoder().decode(CreatePaymentIntentBody.self, from: """
            {"amount_minor_units": \(amount), "currency": "\(currency)", "purpose": "\(purpose.rawValue)"}
        """.data(using: .utf8)!)

        return try await client.request(method: "POST", path: "/v1/payment-intents", body: request)
    }

    public func get(id: String) async throws -> PaymentIntent {
        try await client.request(method: "GET", path: "/v1/payment-intents/\(id)")
    }

    public func authorize(
        id: String,
        paymentMethodToken: String,
        cardScheme: String
    ) async throws -> PaymentIntent {
        let body = AuthorizeBody(paymentMethodToken: paymentMethodToken, cardScheme: cardScheme)
        return try await client.request(method: "POST", path: "/v1/payment-intents/\(id)/authorize", body: body)
    }

    public func capture(id: String, amount: Int? = nil) async throws -> PaymentIntent {
        let body = CaptureBody(amount: amount)
        return try await client.request(method: "POST", path: "/v1/payment-intents/\(id)/capture", body: body)
    }

    public func refund(id: String, amount: Int) async throws -> PaymentIntent {
        let body = RefundBody(amount: amount)
        return try await client.request(method: "POST", path: "/v1/payment-intents/\(id)/refund", body: body)
    }

    public func `void`(id: String) async throws -> PaymentIntent {
        try await client.request(method: "POST", path: "/v1/payment-intents/\(id)/void", body: EmptyBody())
    }
}

// MARK: - Models

public struct PaymentIntent: Codable {
    public let id: String
    public let status: String
    public let amount: Int
    public let currency: String
    public let createdAt: String

    enum CodingKeys: String, CodingKey {
        case id, status, amount, currency
        case createdAt = "created_at"
    }
}

public enum PaymentPurpose: String {
    case payment
    case cardVerification = "card_verification"
}

// MARK: - Request Bodies

struct CreatePaymentIntentBody: Encodable {
    let amountMinorUnits: Int
    let currency: String
    let purpose: String

    enum CodingKeys: String, CodingKey {
        case amountMinorUnits = "amount_minor_units"
        case currency, purpose
    }
}

struct AuthorizeBody: Encodable {
    let paymentMethodToken: String
    let cardScheme: String

    enum CodingKeys: String, CodingKey {
        case paymentMethodToken = "payment_method_token"
        case cardScheme = "card_scheme"
    }
}

struct CaptureBody: Encodable {
    let amount: Int?
}

struct RefundBody: Encodable {
    let amount: Int
}

struct EmptyBody: Encodable {}

struct ErrorResponse: Codable {
    let detail: String
}

// MARK: - Errors

public enum PaymentOrchestraError: Error {
    case invalidResponse
    case httpError(status: Int, message: String)
    case decodingError(Error)
    case networkError(Error)
}

extension PaymentOrchestraError: LocalizedError {
    public var errorDescription: String? {
        switch self {
        case .invalidResponse:
            return "Invalid response from server"
        case .httpError(let status, let message):
            return "[\(status)] \(message)"
        case .decodingError(let error):
            return "Failed to decode response: \(error.localizedDescription)"
        case .networkError(let error):
            return "Network error: \(error.localizedDescription)"
        }
    }
}

// MARK: - Placeholder Resources

public class GatewayProfilesResource {
    private let client: PaymentOrchestra
    init(client: PaymentOrchestra) { self.client = client }
}

public class RoutingPoliciesResource {
    private let client: PaymentOrchestra
    init(client: PaymentOrchestra) { self.client = client }
}

public class WebhooksResource {
    private let client: PaymentOrchestra
    init(client: PaymentOrchestra) { self.client = client }
}

public class ApiKeysResource {
    private let client: PaymentOrchestra
    init(client: PaymentOrchestra) { self.client = client }
}
