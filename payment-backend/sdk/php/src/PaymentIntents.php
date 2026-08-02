<?php

declare(strict_types=1);

namespace PaymentOrchestra;

/**
 * Payment Intents resource
 */
class PaymentIntents
{
    private Client $client;

    public function __construct(Client $client)
    {
        $this->client = $client;
    }

    /**
     * Create a new payment intent
     */
    public function create(array $params): array
    {
        return $this->client->request('POST', '/v1/payment-intents', $params);
    }

    /**
     * Get a payment intent by ID
     */
    public function get(string $id): array
    {
        return $this->client->request('GET', "/v1/payment-intents/$id");
    }

    /**
     * List payment intents
     */
    public function list(array $params = []): array
    {
        return $this->client->request('GET', '/v1/payment-intents', $params);
    }

    /**
     * Capture a payment intent
     */
    public function capture(string $id, ?int $amount = null): array
    {
        $params = [];
        if ($amount !== null) {
            $params['amount'] = $amount;
        }
        return $this->client->request('POST', "/v1/payment-intents/$id/capture", $params);
    }

    /**
     * Void (cancel) a payment intent
     */
    public function void(string $id): array
    {
        return $this->client->request('POST', "/v1/payment-intents/$id/void");
    }

    /**
     * Refund a payment intent
     */
    public function refund(string $id, ?int $amount = null): array
    {
        $params = [];
        if ($amount !== null) {
            $params['amount'] = $amount;
        }
        return $this->client->request('POST', "/v1/payment-intents/$id/refund", $params);
    }

    /**
     * Authorize a payment intent
     */
    public function authorize(string $id): array
    {
        return $this->client->request('POST', "/v1/payment-intents/$id/authorize");
    }
}
