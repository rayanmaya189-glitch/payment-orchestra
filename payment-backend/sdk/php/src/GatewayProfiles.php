<?php

declare(strict_types=1);

namespace PaymentOrchestra;

/**
 * Gateway Profiles resource
 */
class GatewayProfiles
{
    private Client $client;

    public function __construct(Client $client)
    {
        $this->client = $client;
    }

    /**
     * List all gateway profiles
     */
    public function list(): array
    {
        return $this->client->request('GET', '/v1/gateway-profiles');
    }

    /**
     * Get a gateway profile by ID
     */
    public function get(string $id): array
    {
        return $this->client->request('GET', "/v1/gateway-profiles/$id");
    }

    /**
     * Create a new gateway profile
     */
    public function create(array $params): array
    {
        return $this->client->request('POST', '/v1/gateway-profiles', $params);
    }

    /**
     * Update a gateway profile
     */
    public function update(string $id, array $params): array
    {
        return $this->client->request('PATCH', "/v1/gateway-profiles/$id", $params);
    }

    /**
     * Delete a gateway profile
     */
    public function delete(string $id): void
    {
        $this->client->request('DELETE', "/v1/gateway-profiles/$id");
    }

    /**
     * Test a gateway connection
     */
    public function test(string $id): array
    {
        return $this->client->request('POST', "/v1/gateway-profiles/$id/test");
    }
}
