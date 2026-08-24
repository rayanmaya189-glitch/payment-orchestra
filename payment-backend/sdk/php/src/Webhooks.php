<?php

declare(strict_types=1);

namespace PaymentOrchestra;

/**
 * Webhooks resource
 */
class Webhooks
{
    private Client $client;

    public function __construct(Client $client)
    {
        $this->client = $client;
    }

    /**
     * List all webhook endpoints
     */
    public function list(): array
    {
        return $this->client->request('GET', '/v1/webhooks');
    }

    /**
     * Get a webhook endpoint by ID
     */
    public function get(string $id): array
    {
        return $this->client->request('GET', "/v1/webhooks/$id");
    }

    /**
     * Create a new webhook endpoint
     */
    public function create(array $params): array
    {
        return $this->client->request('POST', '/v1/webhooks', $params);
    }

    /**
     * Delete a webhook endpoint
     */
    public function delete(string $id): void
    {
        $this->client->request('DELETE', "/v1/webhooks/$id");
    }

    /**
     * Send a test webhook event
     */
    public function test(string $id): array
    {
        return $this->client->request('POST', "/v1/webhooks/$id/test");
    }
}
