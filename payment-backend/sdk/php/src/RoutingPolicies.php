<?php

declare(strict_types=1);

namespace PaymentOrchestra;

/**
 * Routing Policies resource
 */
class RoutingPolicies
{
    private Client $client;

    public function __construct(Client $client)
    {
        $this->client = $client;
    }

    /**
     * List all routing policies
     */
    public function list(): array
    {
        return $this->client->request('GET', '/v1/routing-policies');
    }

    /**
     * Get a routing policy by ID
     */
    public function get(string $id): array
    {
        return $this->client->request('GET', "/v1/routing-policies/$id");
    }

    /**
     * Create a new routing policy
     */
    public function create(array $params): array
    {
        return $this->client->request('POST', '/v1/routing-policies', $params);
    }

    /**
     * Activate a routing policy
     */
    public function activate(string $id): array
    {
        return $this->client->request('POST', "/v1/routing-policies/$id/activate");
    }

    /**
     * Deactivate a routing policy
     */
    public function deactivate(string $id): array
    {
        return $this->client->request('POST', "/v1/routing-policies/$id/deactivate");
    }
}
